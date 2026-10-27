package chat

import (
	"context"
	"encoding/base64"
	"fmt"
	"net/http"
	"strings"
	"unicode/utf8"

	"github.com/anthropics/anthropic-sdk-go"
)

type attachmentKind int

const (
	kindImage attachmentKind = iota
	kindPDF
	kindText
)

type classifiedAttachment struct {
	Attachment
	kind      attachmentKind
	mediaType string
}

type fileUploader func(ctx context.Context, attachment classifiedAttachment) (string, error)

func classifyAttachments(attachments []Attachment) ([]classifiedAttachment, error) {
	classified := make([]classifiedAttachment, 0, len(attachments))
	for _, attachment := range attachments {
		item, err := classifyAttachment(attachment)
		if err != nil {
			return nil, fmt.Errorf("%s: %w", attachment.Name, err)
		}
		classified = append(classified, item)
	}
	return classified, nil
}

func classifyAttachment(attachment Attachment) (classifiedAttachment, error) {
	if len(attachment.Data) == 0 {
		return classifiedAttachment{}, ErrUnsupportedAttachment
	}

	sniffed := http.DetectContentType(attachment.Data[:min(len(attachment.Data), SniffLength)])
	mediaType, _, _ := strings.Cut(sniffed, ";")

	if _, isImage := imageMediaTypes[mediaType]; isImage {
		return classifiedAttachment{Attachment: attachment, kind: kindImage, mediaType: mediaType}, nil
	}
	if mediaType == pdfMediaType {
		return classifiedAttachment{Attachment: attachment, kind: kindPDF, mediaType: mediaType}, nil
	}
	if strings.HasPrefix(mediaType, "text/") && utf8.Valid(attachment.Data) {
		return classifiedAttachment{Attachment: attachment, kind: kindText, mediaType: textMediaType}, nil
	}
	return classifiedAttachment{}, ErrUnsupportedAttachment
}

func buildAttachmentBlocks(
	ctx context.Context,
	attachments []classifiedAttachment,
	upload fileUploader,
) ([]anthropic.ContentBlockParamUnion, []string, error) {
	blocks := make([]anthropic.ContentBlockParamUnion, 0, len(attachments))
	var uploaded []string
	var inlineBytes int
	for _, attachment := range attachments {
		if inlineBytes+len(attachment.Data) <= InlineAttachmentBudget {
			inlineBytes += len(attachment.Data)
			blocks = append(blocks, inlineBlock(attachment))
			continue
		}
		fileID, err := upload(ctx, attachment)
		if err != nil {
			return nil, uploaded, fmt.Errorf("upload %s: %w", attachment.Name, err)
		}
		uploaded = append(uploaded, fileID)
		blocks = append(blocks, fileBlock(attachment.kind, fileID))
	}
	return blocks, uploaded, nil
}

func inlineBlock(attachment classifiedAttachment) anthropic.ContentBlockParamUnion {
	switch attachment.kind {
	case kindImage:
		return anthropic.NewImageBlockBase64(attachment.mediaType, base64.StdEncoding.EncodeToString(attachment.Data))
	case kindPDF:
		return anthropic.NewDocumentBlock(anthropic.Base64PDFSourceParam{Data: base64.StdEncoding.EncodeToString(attachment.Data)})
	default:
		return anthropic.NewTextBlock(fmt.Sprintf(TextAttachmentFmt, attachment.Name, attachment.Data))
	}
}

func fileBlock(kind attachmentKind, fileID string) anthropic.ContentBlockParamUnion {
	if kind == kindImage {
		return anthropic.NewImageBlock(anthropic.FileImageSourceParam{FileID: fileID})
	}
	return anthropic.NewDocumentBlock(anthropic.FileDocumentSourceParam{FileID: fileID})
}
