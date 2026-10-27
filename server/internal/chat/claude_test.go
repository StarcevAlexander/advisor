package chat

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/anthropics/anthropic-sdk-go/option"
)

const testPNG = "\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89"

func newFakeAnthropic(t *testing.T, responseBody string) (*httptest.Server, *map[string]any) {
	t.Helper()
	captured := map[string]any{}
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, err := io.ReadAll(r.Body)
		if err != nil {
			t.Errorf("read body: %v", err)
		}
		if err := json.Unmarshal(body, &captured); err != nil {
			t.Errorf("decode body: %v", err)
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(responseBody))
	}))
	t.Cleanup(server.Close)
	return server, &captured
}

func textResponse(text, stopReason string) string {
	content := `[{"type":"text","text":` + quote(text) + `}]`
	return `{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-5-5","content":` + content +
		`,"stop_reason":"` + stopReason + `","usage":{"input_tokens":1,"output_tokens":1}}`
}

func quote(value string) string {
	encoded, _ := json.Marshal(value)
	return string(encoded)
}

func TestClaudeReply(t *testing.T) {
	server, captured := newFakeAnthropic(t, textResponse("  Берите второй вариант.  ", "end_turn"))
	claude := NewClaude("test-key", "claude-sonnet-5-5", option.WithBaseURL(server.URL), option.WithMaxRetries(0))

	reply, err := claude.Reply(context.Background(), Conversation{
		History: []Turn{
			{Role: RoleAssistant, Text: "приветствие"},
			{Role: RoleUser, Text: "привет"},
			{Role: RoleAssistant, Text: "здравствуйте"},
		},
		Text:        "какой вариант выбрать?",
		Attachments: []Attachment{{Name: "pic.png", Data: []byte(testPNG)}},
	})
	if err != nil {
		t.Fatalf("Reply: %v", err)
	}
	if reply != "Берите второй вариант." {
		t.Errorf("reply = %q", reply)
	}

	if captured == nil || (*captured)["model"] != "claude-sonnet-5-5" {
		t.Fatalf("model = %v", (*captured)["model"])
	}
	outputConfig, _ := (*captured)["output_config"].(map[string]any)
	if outputConfig["effort"] != "medium" {
		t.Errorf("effort = %v", outputConfig["effort"])
	}
	messages, _ := (*captured)["messages"].([]any)
	if len(messages) != 3 {
		t.Fatalf("messages = %d, want 3 (leading assistant turn dropped)", len(messages))
	}
	last, _ := messages[2].(map[string]any)
	blocks, _ := last["content"].([]any)
	if len(blocks) != 2 {
		t.Fatalf("last message blocks = %d, want image + text", len(blocks))
	}
	if first, _ := blocks[0].(map[string]any); first["type"] != "image" {
		t.Errorf("first block type = %v", first["type"])
	}
}

func TestClaudeReplyRefusal(t *testing.T) {
	server, _ := newFakeAnthropic(t, textResponse("", "refusal"))
	claude := NewClaude("test-key", "claude-sonnet-5-5", option.WithBaseURL(server.URL), option.WithMaxRetries(0))

	reply, err := claude.Reply(context.Background(), Conversation{Text: "вопрос"})
	if err != nil {
		t.Fatalf("Reply: %v", err)
	}
	if reply != RefusalReply {
		t.Errorf("reply = %q", reply)
	}
}

func TestClaudeReplyEmpty(t *testing.T) {
	server, _ := newFakeAnthropic(t, textResponse("   ", "end_turn"))
	claude := NewClaude("test-key", "claude-sonnet-5-5", option.WithBaseURL(server.URL), option.WithMaxRetries(0))

	_, err := claude.Reply(context.Background(), Conversation{Text: "вопрос"})
	if !errors.Is(err, ErrEmptyReply) {
		t.Errorf("err = %v, want ErrEmptyReply", err)
	}
}

func TestClaudeReplyUpstreamError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte(`{"type":"error","error":{"type":"invalid_request_error","message":"bad"}}`))
	}))
	t.Cleanup(server.Close)
	claude := NewClaude("test-key", "claude-sonnet-5-5", option.WithBaseURL(server.URL), option.WithMaxRetries(0))

	_, err := claude.Reply(context.Background(), Conversation{Text: "вопрос"})
	if err == nil || !strings.Contains(err.Error(), "claude request") {
		t.Errorf("err = %v", err)
	}
}

func TestClaudeReplyUploadsOversizedAttachment(t *testing.T) {
	var uploads, deletes int
	var message map[string]any
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		switch {
		case r.Method == http.MethodPost && r.URL.Path == "/v1/files":
			uploads++
			if r.FormValue("expires_in_seconds") != "3600" {
				t.Errorf("expires_in_seconds = %q", r.FormValue("expires_in_seconds"))
			}
			_, _ = w.Write([]byte(`{"id":"file_big","type":"file","filename":"big.pdf","mime_type":"application/pdf","size_bytes":1,"created_at":"2026-10-27T00:00:00Z"}`))
		case r.Method == http.MethodDelete && r.URL.Path == "/v1/files/file_big":
			deletes++
			_, _ = w.Write([]byte(`{"id":"file_big","type":"file_deleted"}`))
		case r.Method == http.MethodPost && r.URL.Path == "/v1/messages":
			if err := json.NewDecoder(r.Body).Decode(&message); err != nil {
				t.Errorf("decode body: %v", err)
			}
			_, _ = w.Write([]byte(textResponse("Прочитал.", "end_turn")))
		default:
			t.Errorf("unexpected %s %s", r.Method, r.URL.Path)
			w.WriteHeader(http.StatusNotFound)
		}
	}))
	t.Cleanup(server.Close)
	claude := NewClaude("test-key", "claude-sonnet-5-5", option.WithBaseURL(server.URL), option.WithMaxRetries(0))

	bigPDF := append([]byte("%PDF-1.4\n"), make([]byte, InlineAttachmentBudget)...)
	reply, err := claude.Reply(context.Background(), Conversation{
		Text: "что в документах?",
		Attachments: []Attachment{
			{Name: "pic.png", Data: []byte(testPNG)},
			{Name: "big.pdf", Data: bigPDF},
		},
	})
	if err != nil {
		t.Fatalf("Reply: %v", err)
	}
	if reply != "Прочитал." {
		t.Errorf("reply = %q", reply)
	}
	if uploads != 1 || deletes != 1 {
		t.Errorf("uploads = %d, deletes = %d, want 1 and 1", uploads, deletes)
	}

	messages, _ := message["messages"].([]any)
	last, _ := messages[len(messages)-1].(map[string]any)
	blocks, _ := last["content"].([]any)
	if len(blocks) != 3 {
		t.Fatalf("blocks = %d, want image + document + text", len(blocks))
	}
	image, _ := blocks[0].(map[string]any)
	imageSource, _ := image["source"].(map[string]any)
	if imageSource["type"] != "base64" {
		t.Errorf("small image source = %v, want base64", imageSource["type"])
	}
	document, _ := blocks[1].(map[string]any)
	documentSource, _ := document["source"].(map[string]any)
	if document["type"] != "document" || documentSource["type"] != "file" || documentSource["file_id"] != "file_big" {
		t.Errorf("big pdf block = %v", document)
	}
}

func TestClaudeReplyUnsupportedAttachment(t *testing.T) {
	claude := NewClaude("test-key", "claude-sonnet-5-5", option.WithMaxRetries(0))

	_, err := claude.Reply(context.Background(), Conversation{
		Text:        "вопрос",
		Attachments: []Attachment{{Name: "a.bin", Data: []byte{0x00, 0x01, 0x02, 0xff, 0xfe}}},
	})
	if !errors.Is(err, ErrUnsupportedAttachment) {
		t.Errorf("err = %v, want ErrUnsupportedAttachment", err)
	}
}
