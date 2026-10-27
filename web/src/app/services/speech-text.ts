import {
  SPEECH_LINE_EDGES_PATTERN,
  SPEECH_LINK_PATTERN,
  SPEECH_QUOTE_LINE_PATTERN,
  SPEECH_SILENT_PATTERN,
  SPEECH_SPACES_PATTERN,
  SPEECH_URL_PATTERN,
  SPEECH_URL_REPLACEMENT,
} from '../constants';

export function toSpeechText(text: string): string {
  return text
    .replace(SPEECH_LINK_PATTERN, '$1')
    .replace(SPEECH_URL_PATTERN, SPEECH_URL_REPLACEMENT)
    .replace(SPEECH_QUOTE_LINE_PATTERN, '')
    .replace(SPEECH_SILENT_PATTERN, ' ')
    .replace(SPEECH_SPACES_PATTERN, ' ')
    .replace(SPEECH_LINE_EDGES_PATTERN, '')
    .trim();
}
