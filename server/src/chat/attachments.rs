use base64::Engine;

use crate::zai::{ContentPart, FileParam, ImageUrlParam};

use super::constants::{INLINE_ATTACHMENT_BUDGET, SNIFF_LENGTH, TEXT_ATTACHMENT_FORMAT};
use super::models::{Attachment, ChatError};

const PDF_MEDIA_TYPE: &str = "application/pdf";

#[derive(Debug, Clone, PartialEq)]
pub enum AttachmentKind {
    Image(&'static str),
    Pdf,
    Text(String),
}

#[derive(Debug, Clone)]
pub struct ClassifiedAttachment {
    pub name: String,
    pub data: Vec<u8>,
    pub kind: AttachmentKind,
}

impl ClassifiedAttachment {
    pub fn is_visual(&self) -> bool {
        !matches!(self.kind, AttachmentKind::Text(_))
    }
}

pub fn classify_attachments(
    attachments: &[Attachment],
) -> Result<Vec<ClassifiedAttachment>, ChatError> {
    attachments.iter().map(classify_attachment).collect()
}

pub fn classify_supported(attachments: &[Attachment]) -> Vec<ClassifiedAttachment> {
    attachments
        .iter()
        .filter_map(|a| classify_attachment(a).ok())
        .collect()
}

pub fn classify_attachment(attachment: &Attachment) -> Result<ClassifiedAttachment, ChatError> {
    if attachment.data.is_empty() {
        return Err(ChatError::UnsupportedAttachment);
    }
    let sniff = &attachment.data[..attachment.data.len().min(SNIFF_LENGTH)];

    if let Some(kind) = infer::get(sniff) {
        match kind.mime_type() {
            "image/jpeg" | "image/png" | "image/gif" | "image/webp" => {
                let media_type = supported_image_media(kind.mime_type());
                return Ok(ClassifiedAttachment {
                    name: attachment.name.clone(),
                    data: attachment.data.clone(),
                    kind: AttachmentKind::Image(media_type),
                });
            }
            PDF_MEDIA_TYPE => {
                return Ok(ClassifiedAttachment {
                    name: attachment.name.clone(),
                    data: attachment.data.clone(),
                    kind: AttachmentKind::Pdf,
                });
            }
            _ => {}
        }
    }

    if is_texty(sniff) && std::str::from_utf8(&attachment.data).is_ok() {
        let text = String::from_utf8_lossy(&attachment.data).into_owned();
        return Ok(ClassifiedAttachment {
            name: attachment.name.clone(),
            data: attachment.data.clone(),
            kind: AttachmentKind::Text(text),
        });
    }
    Err(ChatError::UnsupportedAttachment)
}

fn supported_image_media(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "image/jpeg",
        "image/png" => "image/png",
        "image/gif" => "image/gif",
        "image/webp" => "image/webp",
        _ => unreachable!("checked by caller"),
    }
}

fn is_texty(sniff: &[u8]) -> bool {
    sniff
        .iter()
        .all(|&b| b >= 0x20 || matches!(b, b'\t' | b'\n' | 0x0b | 0x0c | b'\r'))
}

pub fn build_content_parts(
    attachments: &[ClassifiedAttachment],
    budget: &mut usize,
) -> Result<Vec<ContentPart>, ChatError> {
    let mut parts = Vec::with_capacity(attachments.len());
    for attachment in attachments {
        if attachment.data.len() > *budget {
            return Err(ChatError::AttachmentTooLarge);
        }
        *budget -= attachment.data.len();
        parts.push(content_part(attachment));
    }
    Ok(parts)
}

fn content_part(attachment: &ClassifiedAttachment) -> ContentPart {
    match &attachment.kind {
        AttachmentKind::Image(media_type) => ContentPart::ImageUrl {
            image_url: ImageUrlParam {
                url: data_url(media_type, &attachment.data),
            },
        },
        AttachmentKind::Pdf => ContentPart::File {
            file: FileParam {
                file_data: data_url(PDF_MEDIA_TYPE, &attachment.data),
            },
        },
        AttachmentKind::Text(text) => ContentPart::Text {
            text: TEXT_ATTACHMENT_FORMAT
                .replacen("{}", &attachment.name, 1)
                .replacen("{}", text, 1),
        },
    }
}

fn data_url(media_type: &str, data: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    format!("data:{media_type};base64,{encoded}")
}

pub fn default_budget() -> usize {
    INLINE_ATTACHMENT_BUDGET
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89,
    ];
    const TEST_JPEG: &[u8] = &[0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46];
    const TEST_GIF: &[u8] = &[0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00];
    const TEST_WEBP: &[u8] = &[
        0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
    ];
    const TEST_PDF: &[u8] = b"%PDF-1.4\n%%EOF";

    fn attachment(name: &str, data: &[u8]) -> Attachment {
        Attachment {
            name: name.to_string(),
            data: data.to_vec(),
        }
    }

    #[test]
    fn classify_images_pdf_and_text() {
        for (data, expected) in [
            (TEST_PNG, "image/png"),
            (TEST_JPEG, "image/jpeg"),
            (TEST_GIF, "image/gif"),
            (TEST_WEBP, "image/webp"),
        ] {
            let classified = classify_attachment(&attachment("file", data)).unwrap();
            assert_eq!(classified.kind, AttachmentKind::Image(expected));
        }
        let pdf = classify_attachment(&attachment("doc.pdf", TEST_PDF)).unwrap();
        assert_eq!(pdf.kind, AttachmentKind::Pdf);

        let text = classify_attachment(&attachment("note.txt", "привет мир".as_bytes())).unwrap();
        assert_eq!(text.kind, AttachmentKind::Text("привет мир".to_string()));
    }

    #[test]
    fn classify_rejects_empty_and_binary() {
        assert!(matches!(
            classify_attachment(&attachment("empty.bin", &[])),
            Err(ChatError::UnsupportedAttachment)
        ));
        assert!(matches!(
            classify_attachment(&attachment("a.bin", &[0x00, 0x01, 0x02, 0xff, 0xfe])),
            Err(ChatError::UnsupportedAttachment)
        ));
    }

    #[test]
    fn classify_rejects_text_with_control_bytes() {
        let data = b"hello\x00world";
        assert!(matches!(
            classify_attachment(&attachment("bad.txt", data)),
            Err(ChatError::UnsupportedAttachment)
        ));
    }

    #[test]
    fn budget_shared_across_attachments() {
        let mut budget = TEST_PNG.len() + TEST_PDF.len();
        let classified =
            classify_attachments(&[attachment("p", TEST_PNG), attachment("d", TEST_PDF)]).unwrap();
        let parts = build_content_parts(&classified, &mut budget).unwrap();
        assert_eq!(budget, 0);
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn budget_exceeded_returns_too_large() {
        let mut budget = TEST_PNG.len();
        let classified =
            classify_attachments(&[attachment("a", TEST_PNG), attachment("b", TEST_PNG)]).unwrap();
        assert!(matches!(
            build_content_parts(&classified, &mut budget),
            Err(ChatError::AttachmentTooLarge)
        ));
    }

    #[test]
    fn image_part_uses_data_url() {
        let classified = classify_attachment(&attachment("pic.png", TEST_PNG)).unwrap();
        let parts = build_content_parts(&[classified], &mut default_budget()).unwrap();
        let part = serde_json::to_value(&parts[0]).unwrap();
        assert_eq!(part["type"], "image_url");
        assert!(
            part["image_url"]["url"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
    }

    #[test]
    fn pdf_part_uses_file_data_url() {
        let classified = classify_attachment(&attachment("doc.pdf", TEST_PDF)).unwrap();
        let parts = build_content_parts(&[classified], &mut default_budget()).unwrap();
        let part = serde_json::to_value(&parts[0]).unwrap();
        assert_eq!(part["type"], "file");
        assert!(
            part["file"]["file_data"]
                .as_str()
                .unwrap()
                .starts_with("data:application/pdf;base64,")
        );
    }

    #[test]
    fn text_part_wraps_file_name() {
        let classified =
            classify_attachment(&attachment("note.txt", "содержимое".as_bytes())).unwrap();
        let parts = build_content_parts(&[classified], &mut default_budget()).unwrap();
        let part = serde_json::to_value(&parts[0]).unwrap();
        assert_eq!(part["type"], "text");
        assert_eq!(
            part["text"].as_str().unwrap(),
            "Файл «note.txt»:\nсодержимое"
        );
    }
}
