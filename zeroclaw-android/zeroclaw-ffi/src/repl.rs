/*
 * Copyright 2026 ZeroClaw Community
 *
 * Licensed under the MIT License. See LICENSE in the project root.
 */

//! Rhai REPL engine for the terminal screen.
//!
//! Builds a [`rhai::Engine`] with all gateway functions registered as
//! native Rhai calls, then exposes [`eval_repl_inner`] for evaluating
//! arbitrary Rhai expressions from the Kotlin UI layer.
//!
//! Functions that return structured data serialise their results to JSON
//! via `serde_json`. Functions that return `()` produce `"ok"`.
//! Primitives (`String`, `f64`, `bool`, `i64`) are returned directly.

use std::sync::{Mutex, OnceLock};

use rhai::packages::{CorePackage, Package};
use rhai::{Array, Dynamic, Engine, EvalAltResult};

use crate::error::FfiError;
use crate::{
    auth_profiles, events, gateway_client, models, runtime, vision,
};

/// Lazily initialised Rhai engine with all gateway functions registered.
static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

/// Converts an [`FfiError`] into a boxed [`EvalAltResult`] for Rhai.
///
/// Rhai requires `Box<EvalAltResult>` as the error type in all fallible
/// registered functions, so the box is unavoidable.
#[allow(clippy::unnecessary_box_returns)]
fn ffi_err(e: FfiError) -> Box<EvalAltResult> {
    e.to_string().into()
}

/// Serialises a `serde::Serialize` value to a JSON string, or returns
/// a Rhai error on serialisation failure.
fn to_json<T: serde::Serialize>(value: &T) -> Result<String, Box<EvalAltResult>> {
    serde_json::to_string(value)
        .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
}

/// Builds and returns the Rhai engine with all gateway functions registered.
///
/// Uses `Engine::new_raw()` plus `CorePackage` for a minimal footprint.
/// Each registered function calls the corresponding `_inner` function
/// directly to avoid double `catch_unwind` at the FFI boundary.
#[allow(clippy::too_many_lines)]
fn build_engine() -> Engine {
    let mut engine = Engine::new_raw();
    engine.register_global_module(CorePackage::new().as_shared_module());

    // ── Sandbox limits (defence-in-depth) ────────────────────────
    //
    // `Engine::new_raw()` already excludes filesystem, network, and OS
    // access. These limits prevent resource exhaustion from malicious or
    // accidental user expressions (infinite loops, huge allocations).
    engine.set_max_operations(100_000);
    engine.set_max_expr_depths(32, 16);
    engine.set_max_string_size(64 * 1024);
    engine.set_max_array_size(1_024);
    engine.set_max_map_size(256);
    engine.set_max_call_levels(16);

    // ── Lifecycle ────────────────────────────────────────────────

    engine.register_fn("status", || -> Result<String, Box<EvalAltResult>> {
        runtime::get_status_inner().map_err(ffi_err)
    });

    engine.register_fn("version", || -> Result<String, Box<EvalAltResult>> {
        crate::get_version().map_err(ffi_err)
    });

    // ── Messaging ────────────────────────────────────────────────

    engine.register_fn(
        "send",
        |msg: String| -> Result<String, Box<EvalAltResult>> {
            runtime::send_message_inner(msg).map_err(ffi_err)
        },
    );

    engine.register_fn(
        "send_vision",
        |text: String, images: Array, mimes: Array| -> Result<String, Box<EvalAltResult>> {
            let image_data: Vec<String> = images
                .into_iter()
                .map(|v| v.into_string().unwrap_or_default())
                .collect();
            let mime_types: Vec<String> = mimes
                .into_iter()
                .map(|v| v.into_string().unwrap_or_default())
                .collect();
            vision::send_vision_message_inner(text, image_data, mime_types).map_err(ffi_err)
        },
    );

    engine.register_fn(
        "validate_config",
        |toml_str: String| -> Result<String, Box<EvalAltResult>> {
            runtime::validate_config_inner(toml_str).map_err(ffi_err)
        },
    );

    engine.register_fn("config", || -> Result<String, Box<EvalAltResult>> {
        runtime::get_running_config_inner().map_err(ffi_err)
    });

    // ── Channel Binding ─────────────────────────────────────────

    engine.register_fn(
        "bind",
        |channel: String, user_id: String| -> Result<String, Box<EvalAltResult>> {
            let field = runtime::bind_channel_identity_inner(channel.clone(), user_id.clone())
                .map_err(ffi_err)?;
            if field == "already_bound" {
                Ok(format!("{user_id} is already bound to {channel}"))
            } else {
                Ok(format!(
                    "Bound {user_id} to {channel} ({field}). Restart daemon to apply."
                ))
            }
        },
    );

    engine.register_fn(
        "allowlist",
        |channel: String| -> Result<String, Box<EvalAltResult>> {
            let list = runtime::get_channel_allowlist_inner(channel).map_err(ffi_err)?;
            to_json(&list)
        },
    );

    // ── Provider Swap ──────────────────────────────────────────

    engine.register_fn(
        "swap_provider",
        |provider: String, model: String| -> Result<String, Box<EvalAltResult>> {
            runtime::swap_provider_inner(provider, model, None).map_err(ffi_err)?;
            Ok("ok".into())
        },
    );

    // ── Health ────────────────────────────────────────────────────

    engine.register_fn("health", || -> Result<String, Box<EvalAltResult>> {
        let response = gateway_client::gateway_get("/api/health").map_err(ffi_err)?;
        serde_json::to_string(&response)
            .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
    });

    engine.register_fn(
        "health_component",
        |name: String| -> Result<String, Box<EvalAltResult>> {
            let response = gateway_client::gateway_get(&format!("/api/health/{name}"))
                .map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn("doctor", || -> Result<String, Box<EvalAltResult>> {
        runtime::doctor_channels_inner(String::new(), String::new()).map_err(ffi_err)
    });

    engine.register_fn(
        "doctor",
        |config_toml: String, data_dir: String| -> Result<String, Box<EvalAltResult>> {
            runtime::doctor_channels_inner(config_toml, data_dir).map_err(ffi_err)
        },
    );

    // ── Cost ─────────────────────────────────────────────────────

    engine.register_fn("cost", || -> Result<String, Box<EvalAltResult>> {
        let response = gateway_client::gateway_get("/api/cost/summary").map_err(ffi_err)?;
        serde_json::to_string(&response)
            .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
    });

    engine.register_fn("cost_daily", || -> Result<Dynamic, Box<EvalAltResult>> {
        let today = chrono::Utc::now().date_naive();
        let year = chrono::Datelike::year(&today);
        let month = chrono::Datelike::month(&today);
        let day = chrono::Datelike::day(&today);
        let response = gateway_client::gateway_get(&format!("/api/cost/daily/{year}/{month}/{day}"))
            .map_err(ffi_err)?;
        let value = response
            .as_f64()
            .ok_or_else(|| -> Box<EvalAltResult> { "invalid cost value".into() })?;
        Ok(Dynamic::from_float(value))
    });

    engine.register_fn(
        "cost_daily",
        |year: i64, month: i64, day: i64| -> Result<Dynamic, Box<EvalAltResult>> {
            let response = gateway_client::gateway_get(&format!("/api/cost/daily/{year}/{month}/{day}"))
                .map_err(ffi_err)?;
            let value = response
                .as_f64()
                .ok_or_else(|| -> Box<EvalAltResult> { "invalid cost value".into() })?;
            Ok(Dynamic::from_float(value))
        },
    );

    engine.register_fn("cost_monthly", || -> Result<Dynamic, Box<EvalAltResult>> {
        let now = chrono::Utc::now();
        let year = chrono::Datelike::year(&now);
        let month = chrono::Datelike::month(&now);
        let response = gateway_client::gateway_get(&format!("/api/cost/monthly/{year}/{month}"))
            .map_err(ffi_err)?;
        let value = response
            .as_f64()
            .ok_or_else(|| -> Box<EvalAltResult> { "invalid cost value".into() })?;
        Ok(Dynamic::from_float(value))
    });

    engine.register_fn(
        "cost_monthly",
        |year: i64, month: i64| -> Result<Dynamic, Box<EvalAltResult>> {
            let response = gateway_client::gateway_get(&format!("/api/cost/monthly/{year}/{month}"))
                .map_err(ffi_err)?;
            let value = response
                .as_f64()
                .ok_or_else(|| -> Box<EvalAltResult> { "invalid cost value".into() })?;
            Ok(Dynamic::from_float(value))
        },
    );

    engine.register_fn(
        "budget",
        |estimated: f64| -> Result<String, Box<EvalAltResult>> {
            let body = serde_json::json!({ "estimated_cost_usd": estimated });
            let response = gateway_client::gateway_post("/api/budget/check", &body).map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    // ── Events ───────────────────────────────────────────────────

    engine.register_fn(
        "events",
        |limit: i64| -> Result<String, Box<EvalAltResult>> {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let result = events::get_recent_events_inner(limit as u32).map_err(ffi_err)?;
            Ok(result)
        },
    );

    // ── Cron ─────────────────────────────────────────────────────

    engine.register_fn("cron_list", || -> Result<String, Box<EvalAltResult>> {
        let response = gateway_client::gateway_get("/api/cron").map_err(ffi_err)?;
        serde_json::to_string(&response)
            .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
    });

    engine.register_fn(
        "cron_get",
        |id: String| -> Result<String, Box<EvalAltResult>> {
            let response = gateway_client::gateway_get(&format!("/api/cron/{id}")).map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn(
        "cron_add",
        |expression: String, command: String| -> Result<String, Box<EvalAltResult>> {
            let body = serde_json::json!({
                "expression": expression,
                "command": command,
            });
            let response = gateway_client::gateway_post("/api/cron", &body).map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn(
        "cron_oneshot",
        |delay: String, command: String| -> Result<String, Box<EvalAltResult>> {
            let body = serde_json::json!({
                "delay": delay,
                "command": command,
            });
            let response = gateway_client::gateway_post("/api/cron/oneshot", &body).map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn(
        "cron_add_at",
        |timestamp: String, command: String| -> Result<String, Box<EvalAltResult>> {
            let body = serde_json::json!({
                "timestamp": timestamp,
                "command": command,
            });
            let response = gateway_client::gateway_post("/api/cron/at", &body).map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn(
        "cron_add_every",
        |ms: i64, command: String| -> Result<String, Box<EvalAltResult>> {
            let body = serde_json::json!({
                "interval_ms": ms,
                "command": command,
            });
            let response = gateway_client::gateway_post("/api/cron/every", &body).map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn(
        "cron_remove",
        |id: String| -> Result<String, Box<EvalAltResult>> {
            gateway_client::gateway_delete(&format!("/api/cron/{id}")).map_err(ffi_err)?;
            Ok("ok".into())
        },
    );

    engine.register_fn(
        "cron_pause",
        |id: String| -> Result<String, Box<EvalAltResult>> {
            let body = serde_json::json!({});
            gateway_client::gateway_post(&format!("/api/cron/{id}/pause"), &body).map_err(ffi_err)?;
            Ok("ok".into())
        },
    );

    engine.register_fn(
        "cron_resume",
        |id: String| -> Result<String, Box<EvalAltResult>> {
            let body = serde_json::json!({});
            gateway_client::gateway_post(&format!("/api/cron/{id}/resume"), &body).map_err(ffi_err)?;
            Ok("ok".into())
        },
    );

    // ── Skills ───────────────────────────────────────────────────

    engine.register_fn("skills", || -> Result<String, Box<EvalAltResult>> {
        let response = gateway_client::gateway_get("/api/skills").map_err(ffi_err)?;
        serde_json::to_string(&response)
            .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
    });

    engine.register_fn(
        "skill_tools",
        |name: String| -> Result<String, Box<EvalAltResult>> {
            let response = gateway_client::gateway_get(&format!("/api/skills/{name}/tools"))
                .map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn(
        "skill_install",
        |source: String| -> Result<String, Box<EvalAltResult>> {
            let body = serde_json::json!({ "source": source });
            gateway_client::gateway_post("/api/skills/install", &body).map_err(ffi_err)?;
            Ok("ok".into())
        },
    );

    engine.register_fn(
        "skill_remove",
        |name: String| -> Result<String, Box<EvalAltResult>> {
            gateway_client::gateway_delete(&format!("/api/skills/{name}")).map_err(ffi_err)?;
            Ok("ok".into())
        },
    );

    // ── Tools ────────────────────────────────────────────────────

    engine.register_fn("tools", || -> Result<String, Box<EvalAltResult>> {
        let response = gateway_client::gateway_get("/api/tools").map_err(ffi_err)?;
        serde_json::to_string(&response)
            .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
    });

    // ── Memory ───────────────────────────────────────────────────

    engine.register_fn(
        "memories",
        |limit: i64| -> Result<String, Box<EvalAltResult>> {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let response = gateway_client::gateway_get(&format!("/api/memories?limit={}", limit as u32))
                .map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn(
        "memories_by_category",
        |category: String, limit: i64| -> Result<String, Box<EvalAltResult>> {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let body = serde_json::json!({
                "category": category,
                "limit": limit as u32,
            });
            let response = gateway_client::gateway_post("/api/memories/by-category", &body)
                .map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn(
        "memory_recall",
        |query: String, limit: i64| -> Result<String, Box<EvalAltResult>> {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let body = serde_json::json!({
                "query": query,
                "limit": limit as u32,
            });
            let response = gateway_client::gateway_post("/api/memories/recall", &body)
                .map_err(ffi_err)?;
            serde_json::to_string(&response)
                .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
        },
    );

    engine.register_fn(
        "memory_forget",
        |key: String| -> Result<bool, Box<EvalAltResult>> {
            let body = serde_json::json!({ "key": key });
            let response = gateway_client::gateway_post("/api/memories/forget", &body)
                .map_err(ffi_err)?;
            Ok(response.as_bool().unwrap_or(false))
        },
    );

    engine.register_fn("memory_count", || -> Result<i64, Box<EvalAltResult>> {
        let response = gateway_client::gateway_get("/api/memories/count").map_err(ffi_err)?;
        let count = response
            .as_u64()
            .ok_or_else(|| -> Box<EvalAltResult> { "invalid count value".into() })?;
        #[allow(clippy::cast_possible_wrap)]
        Ok(count as i64)
    });

    // ── Emergency Stop ──────────────────────────────────────────

    engine.register_fn("estop", || -> Result<String, Box<EvalAltResult>> {
        crate::estop::engage_estop_inner().map_err(ffi_err)?;
        Ok("ok".into())
    });

    engine.register_fn("estop_status", || -> Result<String, Box<EvalAltResult>> {
        let s = crate::estop::get_estop_status_inner().map_err(ffi_err)?;
        serde_json::to_string(&serde_json::json!({
            "engaged": s.engaged,
            "engaged_at_ms": s.engaged_at_ms,
        }))
        .map_err(|e| -> Box<EvalAltResult> { format!("serialisation failed: {e}").into() })
    });

    engine.register_fn("estop_resume", || -> Result<String, Box<EvalAltResult>> {
        crate::estop::resume_estop_inner().map_err(ffi_err)?;
        Ok("ok".into())
    });

    // ── Traces ──────────────────────────────────────────────────

    engine.register_fn(
        "traces",
        |limit: i64| -> Result<String, Box<EvalAltResult>> {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            crate::traces::query_traces_inner(None, None, limit as u32).map_err(ffi_err)
        },
    );

    engine.register_fn(
        "traces_filter",
        |filter: String, limit: i64| -> Result<String, Box<EvalAltResult>> {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            crate::traces::query_traces_inner(Some(filter), None, limit as u32).map_err(ffi_err)
        },
    );

    // ── Auth Profiles ──────────────────────────────────────────

    engine.register_fn("auth_list", || -> Result<String, Box<EvalAltResult>> {
        let profiles = auth_profiles::list_auth_profiles_inner().map_err(ffi_err)?;
        let json: Vec<serde_json::Value> = profiles
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "provider": p.provider,
                    "kind": p.kind,
                    "active": p.is_active,
                })
            })
            .collect();
        to_json(&json)
    });

    engine.register_fn(
        "auth_remove",
        |provider: String, profile_name: String| -> Result<String, Box<EvalAltResult>> {
            auth_profiles::remove_auth_profile_inner(provider, profile_name).map_err(ffi_err)?;
            Ok("ok".into())
        },
    );

    // ── Model Discovery ──────────────────────────────────────────

    engine.register_fn(
        "models",
        |provider: String| -> Result<String, Box<EvalAltResult>> {
            models::discover_models_inner(provider, String::new(), None).map_err(ffi_err)
        },
    );

    engine.register_fn(
        "models_with_key",
        |provider: String, api_key: String| -> Result<String, Box<EvalAltResult>> {
            models::discover_models_inner(provider, api_key, None).map_err(ffi_err)
        },
    );

    engine.register_fn(
        "models_full",
        |provider: String,
         api_key: String,
         base_url: String|
         -> Result<String, Box<EvalAltResult>> {
            let url = if base_url.is_empty() {
                None
            } else {
                Some(base_url)
            };
            models::discover_models_inner(provider, api_key, url).map_err(ffi_err)
        },
    );

    engine
}

/// Evaluates a Rhai expression against the shared engine.
///
/// Initialises the engine on first call. The `Dynamic` result is
/// converted to a `String` according to these rules:
///
/// - Strings are returned as-is.
/// - Booleans, integers, and floats are converted via `to_string()`.
/// - Unit `()` becomes `"ok"`.
/// - All other types use `to_string()` as a fallback.
///
/// # Errors
///
/// Returns [`FfiError::SpawnError`] if the engine mutex is poisoned or
/// if the Rhai evaluation produces an error.
pub(crate) fn eval_repl_inner(expression: String) -> Result<String, FfiError> {
    let engine_mutex = ENGINE.get_or_init(|| Mutex::new(build_engine()));
    let engine = engine_mutex.lock().map_err(|_| FfiError::StateCorrupted {
        detail: "REPL engine mutex poisoned".into(),
    })?;

    let result: Dynamic =
        engine
            .eval::<Dynamic>(&expression)
            .map_err(|e| FfiError::SpawnError {
                detail: format!("eval error: {e}"),
            })?;

    Ok(dynamic_to_string(result))
}

/// Converts a Rhai [`Dynamic`] value to a display string.
fn dynamic_to_string(value: Dynamic) -> String {
    if value.is_unit() {
        return "ok".into();
    }
    if let Ok(s) = value.clone().into_string() {
        return s;
    }
    value.to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_arithmetic() {
        let result = eval_repl_inner("2 + 3".into()).unwrap();
        assert_eq!(result, "5");
    }

    #[test]
    fn test_eval_string_literal() {
        let result = eval_repl_inner(r#""hello""#.into()).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_eval_boolean() {
        let result = eval_repl_inner("true".into()).unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_eval_unit() {
        let result = eval_repl_inner("let x = 1;".into()).unwrap();
        assert_eq!(result, "ok");
    }

    #[test]
    fn test_eval_float() {
        let result = eval_repl_inner("1.5 + 2.5".into()).unwrap();
        assert_eq!(result, "4.0");
    }

    #[test]
    fn test_eval_syntax_error() {
        let result = eval_repl_inner("{{{{invalid".into());
        assert!(result.is_err());
        match result.unwrap_err() {
            FfiError::SpawnError { detail } => {
                assert!(detail.contains("eval error"));
            }
            other => panic!("expected SpawnError, got {other:?}"),
        }
    }

    #[test]
    fn test_version_returns_string() {
        let result = eval_repl_inner("version()".into()).unwrap();
        assert_eq!(result, "0.0.37");
    }

    #[test]
    fn test_status_returns_json() {
        let result = eval_repl_inner("status()".into()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.get("daemon_running").is_some());
    }

    #[test]
    fn test_validate_config_valid() {
        let result =
            eval_repl_inner(r#"validate_config("default_temperature = 0.7\n")"#.into()).unwrap();
        assert!(
            result.is_empty(),
            "expected empty for valid config, got: {result}"
        );
    }

    #[test]
    fn test_validate_config_invalid() {
        let result = eval_repl_inner(r#"validate_config("not valid {{{{")"#.into()).unwrap();
        assert!(
            !result.is_empty(),
            "expected error message for invalid config"
        );
    }

    #[test]
    fn test_send_not_running() {
        let result = eval_repl_inner(r#"send("hello")"#.into());
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_count_not_running() {
        let result = eval_repl_inner("memory_count()".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_dynamic_to_string_unit() {
        assert_eq!(dynamic_to_string(Dynamic::UNIT), "ok");
    }

    #[test]
    fn test_dynamic_to_string_string() {
        assert_eq!(dynamic_to_string(Dynamic::from("abc".to_string())), "abc");
    }

    #[test]
    fn test_dynamic_to_string_int() {
        assert_eq!(dynamic_to_string(Dynamic::from(42_i64)), "42");
    }

    #[test]
    fn test_dynamic_to_string_bool() {
        assert_eq!(dynamic_to_string(Dynamic::from(true)), "true");
    }

    #[test]
    fn test_dynamic_to_string_float() {
        let result = dynamic_to_string(Dynamic::from_float(1.23));
        assert!(result.starts_with("1.23"));
    }

    #[test]
    fn test_sandbox_blocks_infinite_loop() {
        let result = eval_repl_inner("loop { }".into());
        assert!(
            result.is_err(),
            "infinite loop must be terminated by sandbox"
        );
    }

    #[test]
    fn test_sandbox_blocks_deep_recursion() {
        let expr = r"fn recurse(n) { recurse(n + 1) } recurse(0)";
        let result = eval_repl_inner(expr.into());
        assert!(
            result.is_err(),
            "deep recursion must be terminated by sandbox"
        );
    }

    #[test]
    fn test_repl_bind_no_daemon() {
        let result = eval_repl_inner(r#"bind("telegram", "alice")"#.into());
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_allowlist_no_daemon() {
        let result = eval_repl_inner(r#"allowlist("telegram")"#.into());
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_traces_no_daemon() {
        let result = eval_repl_inner("traces(10)".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_traces_filter_no_daemon() {
        let result = eval_repl_inner(r#"traces_filter("error", 5)"#.into());
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_cron_add_at_no_daemon() {
        let result = eval_repl_inner(r#"cron_add_at("2026-12-31T23:59:59Z", "echo at")"#.into());
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_cron_add_every_no_daemon() {
        let result = eval_repl_inner(r#"cron_add_every(60000, "echo every")"#.into());
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_auth_list_no_daemon() {
        let result = eval_repl_inner("auth_list()".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_auth_remove_no_daemon() {
        let result = eval_repl_inner(r#"auth_remove("openai", "default")"#.into());
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_models_anthropic() {
        let result = eval_repl_inner(r#"models("anthropic")"#.into()).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert!(!parsed.is_empty());
    }

    #[test]
    fn test_repl_swap_provider_no_daemon() {
        let result = eval_repl_inner(r#"swap_provider("anthropic", "claude-sonnet-4")"#.into());
        assert!(result.is_err());
    }

    #[test]
    fn test_repl_models_with_key_anthropic() {
        let result = eval_repl_inner(r#"models_with_key("anthropic", "")"#.into()).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert!(!parsed.is_empty());
    }
}
