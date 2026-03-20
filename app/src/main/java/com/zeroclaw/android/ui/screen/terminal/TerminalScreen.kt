/*
 * Copyright 2026 ZeroClaw Community
 *
 * Licensed under the MIT License. See LICENSE in the project root.
 */

@file:Suppress("TooManyFunctions")

package com.zeroclaw.android.ui.screen.terminal

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.outlined.AttachFile
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.zeroclaw.android.ZeroClawApplication
import com.zeroclaw.android.model.ProcessedImage
import com.zeroclaw.android.model.ServiceState
import com.zeroclaw.android.ui.component.LoadingIndicator
import com.zeroclaw.android.ui.theme.TerminalTypography
import com.zeroclaw.android.util.LocalPowerSaveMode
import kotlinx.coroutines.launch

/** Horizontal padding inside the input bar. */
private const val INPUT_BAR_PADDING_DP = 8

/** Spacing between items in the scrollback. */
private const val BLOCK_SPACING_DP = 4

/** Maximum images per picker invocation. */
private const val MAX_PICKER_IMAGES = 5

/** Autocomplete popup corner radius. */
private const val AUTOCOMPLETE_CORNER_DP = 8

/** Autocomplete popup elevation. */
private const val AUTOCOMPLETE_ELEVATION_DP = 4

/** Autocomplete item vertical padding. */
private const val AUTOCOMPLETE_ITEM_V_PAD_DP = 10

/** Autocomplete item horizontal padding. */
private const val AUTOCOMPLETE_ITEM_H_PAD_DP = 12

/** Small spacing used between elements. */
private const val SMALL_SPACING_DP = 4

/** Status dot size in dp. */
private const val STATUS_DOT_SIZE_DP = 8

/** Header vertical padding. */
private const val HEADER_VERTICAL_PADDING_DP = 8

/** Pending image strip item horizontal padding. */
private const val STRIP_ITEM_H_PAD_DP = 8

/** Pending image strip item vertical padding. */
private const val STRIP_ITEM_V_PAD_DP = 4

/** Pending image strip corner radius. */
private const val STRIP_ITEM_CORNER_DP = 4

/** Dismiss badge size for pending images. */
private const val DISMISS_BADGE_DP = 16

/** Dismiss icon size. */
private const val DISMISS_ICON_DP = 10

/** Loading indicator size in the pending strip. */
private const val PROCESSING_INDICATOR_DP = 14

/**
 * Terminal REPL screen for interacting with the ZeroClaw daemon.
 *
 * Thin stateful wrapper that collects [TerminalViewModel] flows and
 * delegates rendering to [TerminalContent]. Provides the photo picker
 * launcher for image attachments.
 *
 * @param edgeMargin Horizontal padding based on window width size class.
 * @param terminalViewModel The [TerminalViewModel] for terminal state.
 * @param modifier Modifier applied to the root layout.
 */
@Composable
fun TerminalScreen(
    edgeMargin: Dp,
    terminalViewModel: TerminalViewModel = viewModel(),
    modifier: Modifier = Modifier,
) {
    val state by terminalViewModel.state.collectAsStateWithLifecycle()
    val streamingState by terminalViewModel.streamingState.collectAsStateWithLifecycle()
    val app = LocalContext.current.applicationContext as ZeroClawApplication
    val serviceState by app.daemonBridge.serviceState.collectAsStateWithLifecycle()

    TerminalContent(
        state = state,
        streamingState = streamingState,
        serviceState = serviceState,
        onSubmit = terminalViewModel::submitInput,
        onAttachImages = terminalViewModel::attachImages,
        onRemoveImage = terminalViewModel::removeImage,
        onCancelAgent = terminalViewModel::cancelAgentTurn,
        onCheckHealth = terminalViewModel::checkGatewayHealth,
        onListCron = terminalViewModel::listCronJobs,
        onGetCost = terminalViewModel::getCostSummary,
        onListMemories = terminalViewModel::listMemories,
        onListSkills = terminalViewModel::listSkills,
        edgeMargin = edgeMargin,
        modifier = modifier,
    )
}

/**
 * Stateless terminal content composable for testing.
 *
 * Renders the terminal scrollback buffer, input bar, pending image
 * strip, autocomplete overlay, and live agent streaming card. All
 * state is passed in as parameters for deterministic previews and
 * unit tests. Includes a gateway operations panel for quick access
 * to gateway functions (health, cron, cost, memory, skills).
 *
 * @param state Aggregated terminal state snapshot.
 * @param streamingState Live agent session streaming state.
 * @param serviceState Current daemon service lifecycle state.
 * @param onSubmit Callback to submit user input text.
 * @param onAttachImages Callback to attach images from URIs.
 * @param onRemoveImage Callback to remove a pending image by index.
 * @param onCancelAgent Callback to cancel the active agent turn.
 * @param onCheckHealth Callback to invoke gateway health check.
 * @param onListCron Callback to list cron jobs from gateway.
 * @param onGetCost Callback to get cost summary from gateway.
 * @param onListMemories Callback to list memories from gateway.
 * @param onListSkills Callback to list skills from gateway.
 * @param edgeMargin Horizontal padding based on window width size class.
 * @param modifier Modifier applied to the root layout.
 */
@Composable
internal fun TerminalContent(
    state: TerminalState,
    streamingState: StreamingState,
    serviceState: ServiceState,
    onSubmit: (String) -> Unit,
    onAttachImages: (List<Uri>) -> Unit,
    onRemoveImage: (Int) -> Unit,
    onCancelAgent: () -> Unit,
    onCheckHealth: () -> Unit,
    onListCron: () -> Unit,
    onGetCost: () -> Unit,
    onListMemories: () -> Unit,
    onListSkills: () -> Unit,
    edgeMargin: Dp,
    modifier: Modifier = Modifier,
) {
    val listState = rememberLazyListState()
    val snackbarHostState = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    var inputText by remember { mutableStateOf("") }
    val isPowerSave = LocalPowerSaveMode.current

    val photoPickerLauncher =
        rememberLauncherForActivityResult(
            contract = ActivityResultContracts.PickMultipleVisualMedia(MAX_PICKER_IMAGES),
        ) { uris: List<Uri> ->
            if (uris.isNotEmpty()) {
                onAttachImages(uris)
            }
        }

    val isAgentActive =
        streamingState.phase != StreamingPhase.IDLE &&
            streamingState.phase != StreamingPhase.COMPLETE &&
            streamingState.phase != StreamingPhase.CANCELLED &&
            streamingState.phase != StreamingPhase.ERROR
    val isInputDisabled = state.isLoading || isAgentActive

    val stableOnRemove: (Int) -> Unit = remember { { index -> onRemoveImage(index) } }

    val autocompletePrefix by remember {
        derivedStateOf {
            if (inputText.startsWith("/")) {
                inputText.removePrefix("/")
            } else {
                null
            }
        }
    }
    val autocompleteSuggestions by remember {
        derivedStateOf {
            val prefix = autocompletePrefix
            if (prefix != null) {
                CommandRegistry.matches(prefix)
            } else {
                emptyList()
            }
        }
    }

    LaunchedEffect(state.blocks.size, streamingState.phase) {
        if (state.blocks.isNotEmpty() || isAgentActive) {
            if (isPowerSave) {
                listState.scrollToItem(0)
            } else {
                listState.animateScrollToItem(0)
            }
        }
    }

    Box(modifier = modifier.fillMaxSize()) {
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .background(MaterialTheme.colorScheme.surface)
                    .imePadding(),
        ) {
            TerminalHeader(
                serviceState = serviceState,
                modifier = Modifier.padding(horizontal = edgeMargin),
            )

            GatewayOperationsPanel(
                onCheckHealth = onCheckHealth,
                onListCron = onListCron,
                onGetCost = onGetCost,
                onListMemories = onListMemories,
                onListSkills = onListSkills,
                isEnabled = !isInputDisabled,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = edgeMargin, vertical = 8.dp),
            )

            LazyColumn(
                state = listState,
                reverseLayout = true,
                modifier =
                    Modifier
                        .weight(1f)
                        .padding(horizontal = edgeMargin),
                verticalArrangement = Arrangement.spacedBy(BLOCK_SPACING_DP.dp),
            ) {
                if (isAgentActive) {
                    if (streamingState.responseText.isNotEmpty()) {
                        item(key = "streaming-response", contentType = "streaming") {
                            StreamingResponseBlock(
                                text = streamingState.responseText,
                                modifier =
                                    Modifier.padding(
                                        horizontal = AUTOCOMPLETE_ITEM_H_PAD_DP.dp,
                                        vertical = SMALL_SPACING_DP.dp,
                                    ),
                            )
                        }
                    }

                    item(key = "thinking-card", contentType = "thinking") {
                        ThinkingCard(
                            thinkingText = streamingState.thinkingText,
                            visible = true,
                            onCancel = onCancelAgent,
                            activeTools = streamingState.activeTools,
                            toolResults = streamingState.toolResults,
                            progressMessage = streamingState.progressMessage,
                            modifier =
                                Modifier.padding(
                                    horizontal = AUTOCOMPLETE_ITEM_H_PAD_DP.dp,
                                    vertical = SMALL_SPACING_DP.dp,
                                ),
                        )
                    }
                } else if (state.isLoading) {
                    item(key = "spinner", contentType = "spinner") {
                        BrailleSpinner(
                            label = "Thinking\u2026",
                            modifier =
                                Modifier.padding(
                                    horizontal = AUTOCOMPLETE_ITEM_H_PAD_DP.dp,
                                    vertical = SMALL_SPACING_DP.dp,
                                ),
                        )
                    }
                }

                items(
                    items = state.blocks.reversed(),
                    key = { it.id },
                    contentType = { block -> block::class.simpleName },
                ) { block ->
                    val onCopy: (String) -> Unit =
                        remember(block.id) {
                            { text ->
                                copyToClipboard(context, text)
                                scope.launch {
                                    snackbarHostState.showSnackbar("Copied to clipboard")
                                }
                                Unit
                            }
                        }
                    TerminalBlockItem(block = block, onCopy = onCopy)
                }
            }

            if (state.pendingImages.isNotEmpty() || state.isProcessingImages) {
                PendingImagesStrip(
                    images = state.pendingImages,
                    isProcessing = state.isProcessingImages,
                    onRemove = stableOnRemove,
                    modifier = Modifier.padding(horizontal = edgeMargin),
                )
            }

            if (autocompleteSuggestions.isNotEmpty()) {
                AutocompletePopup(
                    suggestions = autocompleteSuggestions,
                    onSelect = { command ->
                        inputText = "/${command.name} "
                    },
                    modifier = Modifier.padding(horizontal = edgeMargin),
                )
            }

            TerminalInputBar(
                value = inputText,
                onValueChange = { inputText = it },
                onSubmit = {
                    onSubmit(inputText)
                    inputText = ""
                },
                onAttach = {
                    photoPickerLauncher.launch(
                        PickVisualMediaRequest(
                            ActivityResultContracts.PickVisualMedia.ImageOnly,
                        ),
                    )
                },
                isLoading = isInputDisabled,
                hasImages = state.pendingImages.isNotEmpty(),
                modifier =
                    Modifier.padding(
                        horizontal = edgeMargin,
                        vertical = INPUT_BAR_PADDING_DP.dp,
                    ),
            )
        }

        SnackbarHost(
            hostState = snackbarHostState,
            modifier = Modifier.align(Alignment.BottomCenter),
        )
    }
}

/**
 * Terminal header row showing service state with a coloured status dot.
 *
 * The dot colour conveys the daemon lifecycle state: green for running,
 * amber for starting or stopping, and red for stopped or error. A live
 * region announcement ensures screen readers report state changes.
 *
 * @param serviceState Current daemon service lifecycle state.
 * @param modifier Modifier applied to the header row.
 */
@Composable
private fun TerminalHeader(
    serviceState: ServiceState,
    modifier: Modifier = Modifier,
) {
    val statusLabel =
        when (serviceState) {
            ServiceState.STOPPED -> "stopped"
            ServiceState.STARTING -> "starting"
            ServiceState.RUNNING -> "running"
            ServiceState.STOPPING -> "stopping"
            ServiceState.ERROR -> "error"
        }
    val dotColor =
        when (serviceState) {
            ServiceState.RUNNING -> MaterialTheme.colorScheme.primary
            ServiceState.STARTING,
            ServiceState.STOPPING,
            -> MaterialTheme.colorScheme.tertiary
            ServiceState.STOPPED,
            ServiceState.ERROR,
            -> MaterialTheme.colorScheme.error
        }

    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier =
            modifier
                .fillMaxWidth()
                .padding(vertical = HEADER_VERTICAL_PADDING_DP.dp)
                .semantics(mergeDescendants = true) {
                    contentDescription = "ZeroClaw Terminal, status: $statusLabel"
                    liveRegion = LiveRegionMode.Polite
                },
    ) {
        Text(
            text = "ZeroClaw Terminal",
            style = TerminalTypography.titleSmall,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.width(SMALL_SPACING_DP.dp))
        Box(
            modifier =
                Modifier
                    .size(STATUS_DOT_SIZE_DP.dp)
                    .background(dotColor, CircleShape),
        )
    }
}

/**
 * Gateway operations panel showing quick-action buttons for common gateway functions.
 *
 * Displays five buttons that trigger gateway API calls through the REPL engine:
 * - Health: Check gateway and daemon health
 * - Cron: List all scheduled cron jobs
 * - Cost: Show cost summary and budget
 * - Memory: List all session memories
 * - Skills: List all available skills
 *
 * The buttons are disabled while a request is in progress to prevent concurrent calls.
 *
 * @param onCheckHealth Callback to check gateway health.
 * @param onListCron Callback to list cron jobs.
 * @param onGetCost Callback to get cost summary.
 * @param onListMemories Callback to list memories.
 * @param onListSkills Callback to list skills.
 * @param isEnabled Whether the buttons should be enabled.
 * @param modifier Modifier applied to the container.
 */
@Composable
private fun GatewayOperationsPanel(
    onCheckHealth: () -> Unit,
    onListCron: () -> Unit,
    onGetCost: () -> Unit,
    onListMemories: () -> Unit,
    onListSkills: () -> Unit,
    isEnabled: Boolean,
    modifier: Modifier = Modifier,
) {
    Surface(
        shape = RoundedCornerShape(8.dp),
        color = MaterialTheme.colorScheme.surfaceVariant,
        tonalElevation = 2.dp,
        modifier = modifier,
    ) {
        Column(
            modifier = Modifier.padding(8.dp),
        ) {
            Text(
                text = "Gateway Operations",
                style = TerminalTypography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(bottom = 4.dp),
            )
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                GatewayActionButton(
                    label = "Health",
                    onClick = onCheckHealth,
                    enabled = isEnabled,
                )
                GatewayActionButton(
                    label = "Cron",
                    onClick = onListCron,
                    enabled = isEnabled,
                )
                GatewayActionButton(
                    label = "Cost",
                    onClick = onGetCost,
                    enabled = isEnabled,
                )
                GatewayActionButton(
                    label = "Memory",
                    onClick = onListMemories,
                    enabled = isEnabled,
                )
                GatewayActionButton(
                    label = "Skills",
                    onClick = onListSkills,
                    enabled = isEnabled,
                )
            }
        }
    }
}

/**
 * A compact action button for gateway operations.
 *
 * Styled as a small, clickable button with minimal vertical padding to fit
 * the gateway operations panel layout.
 *
 * @param label The button label text.
 * @param onClick Callback when the button is tapped.
 * @param enabled Whether the button is enabled.
 * @param modifier Modifier applied to the button container.
 */
@Composable
private fun GatewayActionButton(
    label: String,
    onClick: () -> Unit,
    enabled: Boolean,
    modifier: Modifier = Modifier,
) {
    Surface(
        shape = RoundedCornerShape(4.dp),
        color =
            if (enabled) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.outline
            },
        modifier = modifier,
    ) {
        Box(
            modifier = Modifier
                .clickable(enabled = enabled, onClick = onClick)
                .padding(horizontal = 8.dp, vertical = 4.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = label,
                style = TerminalTypography.labelSmall,
                color =
                    if (enabled) {
                        MaterialTheme.colorScheme.onPrimary
                    } else {
                        MaterialTheme.colorScheme.onSurfaceVariant
                    },
            )
        }
    }
}

/**
 * Input bar with a prompt prefix, text field, attach button, and send button.
 *
 * Uses monospace typography for the terminal aesthetic. The `>` prompt
 * prefix is rendered as leading text within the outlined text field.
 *
 * @param value Current input text.
 * @param onValueChange Callback when text changes.
 * @param onSubmit Callback when the send button is tapped.
 * @param onAttach Callback when the attach button is tapped.
 * @param isLoading Whether a response is in progress (disables send).
 * @param hasImages Whether images are currently attached.
 * @param modifier Modifier applied to the input bar.
 */
@Composable
private fun TerminalInputBar(
    value: String,
    onValueChange: (String) -> Unit,
    onSubmit: () -> Unit,
    onAttach: () -> Unit,
    isLoading: Boolean,
    hasImages: Boolean,
    modifier: Modifier = Modifier,
) {
    val canSend = (value.isNotBlank() || hasImages) && !isLoading

    Row(
        modifier = modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        IconButton(
            onClick = onAttach,
            enabled = !isLoading,
            modifier =
                Modifier.semantics {
                    contentDescription = "Attach images"
                },
        ) {
            Icon(
                Icons.Outlined.AttachFile,
                contentDescription = null,
                tint =
                    if (!isLoading) {
                        MaterialTheme.colorScheme.onSurfaceVariant
                    } else {
                        MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)
                    },
            )
        }
        OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            textStyle =
                TerminalTypography.bodyMedium.copy(
                    color = MaterialTheme.colorScheme.onSurface,
                ),
            prefix = {
                Text(
                    text = "> ",
                    style = TerminalTypography.bodyMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
            },
            placeholder = {
                Text(
                    text = "Type a command or message",
                    style = TerminalTypography.bodyMedium,
                )
            },
            singleLine = true,
            colors =
                OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.primary,
                    unfocusedBorderColor = MaterialTheme.colorScheme.outline,
                ),
            modifier = Modifier.weight(1f),
        )
        Spacer(modifier = Modifier.width(SMALL_SPACING_DP.dp))
        IconButton(
            onClick = onSubmit,
            enabled = canSend,
            modifier =
                Modifier.semantics {
                    contentDescription = "Send"
                },
        ) {
            Icon(
                Icons.AutoMirrored.Filled.Send,
                contentDescription = null,
                tint =
                    if (canSend) {
                        MaterialTheme.colorScheme.primary
                    } else {
                        MaterialTheme.colorScheme.onSurfaceVariant
                    },
            )
        }
    }
}

/**
 * Autocomplete popup showing matching slash commands above the input bar.
 *
 * Each suggestion displays the command name and its description. Tapping
 * a suggestion inserts the command text into the input field.
 *
 * @param suggestions Filtered list of matching commands.
 * @param onSelect Callback when a suggestion is tapped.
 * @param modifier Modifier applied to the popup container.
 */
@Composable
private fun AutocompletePopup(
    suggestions: List<SlashCommand>,
    onSelect: (SlashCommand) -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        shape = RoundedCornerShape(AUTOCOMPLETE_CORNER_DP.dp),
        tonalElevation = AUTOCOMPLETE_ELEVATION_DP.dp,
        modifier = modifier.fillMaxWidth(),
    ) {
        Column {
            for (command in suggestions) {
                Row(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .clickable { onSelect(command) }
                            .padding(
                                horizontal = AUTOCOMPLETE_ITEM_H_PAD_DP.dp,
                                vertical = AUTOCOMPLETE_ITEM_V_PAD_DP.dp,
                            ).semantics {
                                contentDescription =
                                    "/${command.name}: ${command.description}"
                            },
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "/${command.name}",
                        style = TerminalTypography.bodyMedium,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    Spacer(modifier = Modifier.width(INPUT_BAR_PADDING_DP.dp))
                    Text(
                        text = command.description,
                        style = TerminalTypography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

/**
 * Horizontal strip of pending image indicators in terminal aesthetic.
 *
 * Each image is shown as a text label `[filename size]` with a dismiss
 * button, matching the terminal look instead of graphical thumbnails.
 * A processing indicator appears when images are being downscaled.
 *
 * @param images Currently staged images.
 * @param isProcessing Whether images are still being processed.
 * @param onRemove Callback to remove an image by index.
 * @param modifier Modifier applied to the strip.
 */
@Composable
private fun PendingImagesStrip(
    images: List<ProcessedImage>,
    isProcessing: Boolean,
    onRemove: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(SMALL_SPACING_DP.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (isProcessing) {
            LoadingIndicator(modifier = Modifier.size(PROCESSING_INDICATOR_DP.dp))
        }
        for ((index, image) in images.withIndex()) {
            val stableOnRemove = remember(index) { { onRemove(index) } }
            PendingImageChip(
                image = image,
                onRemove = stableOnRemove,
            )
        }
    }
}

/**
 * Terminal-styled chip showing an image filename with a dismiss button.
 *
 * @param image The processed image to display.
 * @param onRemove Callback when the dismiss button is tapped.
 */
@Composable
private fun PendingImageChip(
    image: ProcessedImage,
    onRemove: () -> Unit,
) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier =
            Modifier
                .background(
                    MaterialTheme.colorScheme.surfaceVariant,
                    RoundedCornerShape(STRIP_ITEM_CORNER_DP.dp),
                ).padding(
                    horizontal = STRIP_ITEM_H_PAD_DP.dp,
                    vertical = STRIP_ITEM_V_PAD_DP.dp,
                ),
    ) {
        Text(
            text = "[${image.displayName}]",
            style = TerminalTypography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.width(SMALL_SPACING_DP.dp))
        Box(
            modifier =
                Modifier
                    .size(DISMISS_BADGE_DP.dp)
                    .background(MaterialTheme.colorScheme.error, CircleShape)
                    .clickable(onClick = onRemove)
                    .semantics {
                        contentDescription = "Remove ${image.displayName}"
                    },
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                imageVector = Icons.Filled.Close,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onError,
                modifier = Modifier.size(DISMISS_ICON_DP.dp),
            )
        }
    }
}

/**
 * Streaming response block that renders progressively growing text.
 *
 * Styled identically to [TerminalBlock.Response] blocks but rendered
 * inline during the streaming phase. When the turn completes, this block
 * disappears and a persisted [TerminalBlock.Response] replaces it in
 * the scrollback.
 *
 * @param text Accumulated response tokens so far.
 * @param modifier Modifier applied to the text block.
 */
@Composable
private fun StreamingResponseBlock(
    text: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text = text,
        style = TerminalTypography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurface,
        modifier =
            modifier
                .fillMaxWidth()
                .semantics {
                    contentDescription = "Streaming response"
                    liveRegion = LiveRegionMode.Polite
                },
    )
}

/**
 * Copies the given text to the system clipboard.
 *
 * @param context Android context for system service access.
 * @param text The text to copy.
 */
private fun copyToClipboard(
    context: Context,
    text: String,
) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    val clip = ClipData.newPlainText("Terminal output", text)
    clip.description.extras =
        android.os.PersistableBundle().apply {
            putBoolean("android.content.extra.IS_SENSITIVE", true)
        }
    clipboard.setPrimaryClip(clip)
}
