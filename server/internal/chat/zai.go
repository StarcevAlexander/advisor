package chat

import (
	"context"
	"fmt"
	"log/slog"
	"strings"

	"github.com/anthropics/anthropic-sdk-go"
	"github.com/anthropics/anthropic-sdk-go/option"
)

type Zai struct {
	client anthropic.Client
	model  string
}

func NewZai(apiKey, baseURL, model string, options ...option.RequestOption) *Zai {
	allOptions := append([]option.RequestOption{
		option.WithAuthToken(apiKey),
		option.WithBaseURL(baseURL),
	}, options...)
	return &Zai{
		client: anthropic.NewClient(allOptions...),
		model:  model,
	}
}

func (z *Zai) Reply(ctx context.Context, conversation Conversation) (string, error) {
	classified, err := classifyAttachments(conversation.Attachments)
	if err != nil {
		return "", err
	}

	inlineBudget := InlineAttachmentBudget
	attachmentBlocks, err := buildAttachmentBlocks(classified, &inlineBudget)
	if err != nil {
		return "", err
	}

	history := historyMessages(conversation.History, func(turn Turn) []anthropic.ContentBlockParamUnion {
		blocks, err := buildAttachmentBlocks(classifySupported(turn.Attachments), &inlineBudget)
		if err != nil {
			slog.Warn("history attachments skipped", slog.Any("error", err))
			return nil
		}
		return blocks
	})
	messages := buildMessages(history, conversation.Text, attachmentBlocks)

	response, err := z.client.Messages.New(ctx, anthropic.MessageNewParams{
		Model:     z.model,
		MaxTokens: MaxOutputTokens,
		System:    []anthropic.TextBlockParam{{Text: SystemPrompt}},
		Messages:  messages,
	})
	if err != nil {
		return "", fmt.Errorf("zai request: %w", err)
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
