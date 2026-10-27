package chat

import (
	"bytes"
	"context"
	"fmt"
	"log/slog"
	"strings"

	"github.com/anthropics/anthropic-sdk-go"
	"github.com/anthropics/anthropic-sdk-go/option"
)

type Claude struct {
	client anthropic.Client
	model  string
}

func NewClaude(apiKey, model string, options ...option.RequestOption) *Claude {
	allOptions := append([]option.RequestOption{option.WithAPIKey(apiKey)}, options...)
	return &Claude{
		client: anthropic.NewClient(allOptions...),
		model:  model,
	}
}

func (c *Claude) Reply(ctx context.Context, conversation Conversation) (string, error) {
	classified, err := classifyAttachments(conversation.Attachments)
	if err != nil {
		return "", err
	}

	var uploaded []string
	defer func() { c.deleteFiles(uploaded) }()
	inlineBudget := InlineAttachmentBudget
	attachmentBlocks, fileIDs, err := buildAttachmentBlocks(ctx, classified, c.uploadFile, &inlineBudget)
	uploaded = append(uploaded, fileIDs...)
	if err != nil {
		return "", err
	}

	history := historyMessages(conversation.History, func(turn Turn) []anthropic.ContentBlockParamUnion {
		blocks, fileIDs, err := buildAttachmentBlocks(ctx, classifySupported(turn.Attachments), c.uploadFile, &inlineBudget)
		uploaded = append(uploaded, fileIDs...)
		if err != nil {
			slog.Warn("history attachments skipped", slog.Any("error", err))
			return nil
		}
		return blocks
	})
	messages := buildMessages(history, conversation.Text, attachmentBlocks)

	response, err := c.client.Messages.New(ctx, anthropic.MessageNewParams{
		Model:        c.model,
		MaxTokens:    MaxOutputTokens,
		System:       []anthropic.TextBlockParam{{Text: SystemPrompt}},
		OutputConfig: anthropic.OutputConfigParam{Effort: anthropic.OutputConfigEffortMedium},
		Messages:     messages,
	})
	if err != nil {
		return "", fmt.Errorf("claude request: %w", err)
	}

	if response.StopReason == anthropic.StopReasonRefusal {
		return RefusalReply, nil
	}

	var reply strings.Builder
	for _, block := range response.Content {
		if text, ok := block.AsAny().(anthropic.TextBlock); ok {
			reply.WriteString(text.Text)
		}
	}
	if strings.TrimSpace(reply.String()) == "" {
		return "", ErrEmptyReply
	}
	return strings.TrimSpace(reply.String()), nil
}

func (c *Claude) uploadFile(ctx context.Context, attachment classifiedAttachment) (string, error) {
	metadata, err := c.client.Files.Upload(ctx, anthropic.FileUploadParams{
		File:             anthropic.File(bytes.NewReader(attachment.Data), attachment.Name, attachment.mediaType),
		ExpiresInSeconds: anthropic.Int(UploadedFileTTLSeconds),
	})
	if err != nil {
		return "", err
	}
	return metadata.ID, nil
}

func (c *Claude) deleteFiles(fileIDs []string) {
	if len(fileIDs) == 0 {
		return
	}
	ctx, cancel := context.WithTimeout(context.Background(), FileCleanupTimeout)
	defer cancel()
	for _, fileID := range fileIDs {
		if _, err := c.client.Files.Delete(ctx, fileID, anthropic.FileDeleteParams{}); err != nil {
			slog.Warn("delete uploaded file failed", slog.String("file", fileID), slog.Any("error", err))
		}
	}
}
