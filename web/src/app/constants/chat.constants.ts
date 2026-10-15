import { RecognitionState } from '../models';

export const RECOGNITION_LANG = 'ru-RU';

export const SPEECH_LANG = 'ru-RU';

export const MAX_ATTACHMENT_SIZE_BYTES = 20 * 1024 * 1024;

export const MAX_ATTACHMENTS = 8;

export const ACCEPTED_FILE_TYPES = 'image/*,application/pdf,text/plain,text/markdown,text/csv';

export const MIC_LABELS = {
  idle: 'Нажмите и говорите',
  listening: 'Слушаю… нажмите, чтобы отправить',
  unsupported: 'Голосовой ввод не поддерживается',
  denied: 'Нет доступа к микрофону',
  failed: 'Не расслышал, попробуйте ещё раз',
} as const satisfies Record<RecognitionState, string>;

export const WELCOME_TEXT =
  'Я ваш советчик. Нажмите большую кнопку и скажите, что нужно, или напишите текстом и приложите файл.';
