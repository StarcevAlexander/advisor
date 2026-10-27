export const SPEECH_LINK_PATTERN = /\[([^\]]*)\]\([^)]*\)/g;

export const SPEECH_URL_PATTERN = /https?:\/\/\S+/g;

export const SPEECH_URL_REPLACEMENT = 'ссылка';

export const SPEECH_SILENT_PATTERN = /[\\"'`«»„“”‘’*_#~|]+/g;

export const SPEECH_QUOTE_LINE_PATTERN = /^\s*>+\s?/gm;

export const SPEECH_SPACES_PATTERN = /[ \t]+/g;

export const SPEECH_LINE_EDGES_PATTERN = /^ +| +$/gm;
