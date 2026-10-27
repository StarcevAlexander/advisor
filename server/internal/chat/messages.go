package chat

import (
	"fmt"
	"strings"

	"github.com/anthropics/anthropic-sdk-go"
)

type turnBlocksBuilder func(turn Turn) []anthropic.ContentBlockParamUnion

func buildMessages(history []anthropic.MessageParam, text string, attachmentBlocks []anthropic.ContentBlockParamUnion) []anthropic.MessageParam {
	blocks := attachmentBlocks
	if text := strings.TrimSpace(text); text != "" {
		blocks = append(blocks, anthropic.NewTextBlock(text))
	}
	return append(history, anthropic.NewUserMessage(blocks...))
}

func historyMessages(history []Turn, attachmentBlocks turnBlocksBuilder) []anthropic.MessageParam {
	if len(history) > MaxHistoryTurns {
		history = history[len(history)-MaxHistoryTurns:]
	}

	messages := make([]anthropic.MessageParam, 0, len(history))
	for _, turn := range history {
		text := turnText(turn)
		switch turn.Role {
		case RoleUser:
			blocks := attachmentBlocks(turn)
			if text != "" {
				blocks = append(blocks, anthropic.NewTextBlock(text))
			}
			if len(blocks) == 0 {
				continue
			}
			messages = append(messages, anthropic.NewUserMessage(blocks...))
		case RoleAssistant:
			if text == "" || len(messages) == 0 {
				continue
			}
			messages = append(messages, anthropic.NewAssistantMessage(anthropic.NewTextBlock(text)))
		}
	}
	return messages
}

func turnText(turn Turn) string {
	text := strings.TrimSpace(turn.Text)
	if turn.Role != RoleUser || len(turn.FileNames) == 0 {
		return text
	}
	marker := fmt.Sprintf(HistoryFilesFmt, strings.Join(turn.FileNames, ", "))
	if text == "" {
		return marker
	}
	return marker + "\n" + text
}

func noTurnBlocks(Turn) []anthropic.ContentBlockParamUnion {
	return nil
}
