export const API_DEV_URL = 'http://localhost:8080';

export const API_PRODUCTION_URL = 'https://interceptor.tail969ed0.ts.net:8443';

export const API_CHAT_PATH = '/api/chat';

export const API_CHANGE_PATH = '/api/change';

export const API_CHANGE_PLAN_PATH = '/api/change/plan';

export const API_ROLLBACK_PATH = '/api/rollback';

export const API_CLIENT_ERROR_PATH = '/api/client-error';

export const CONVERSATION_STORAGE_KEY = 'advisor.conversation-id';

export const HTTP_UNAUTHORIZED = 401;

export const HTTP_OFFLINE = 0;

export const ACCESS_PASSWORD_STORAGE_KEY = 'advisor.access-password';

export const API_MESSAGES = {
  notConfigured: 'Адрес сервера советчика не задан',
  unauthorized: 'Неверный пароль. Введите его заново',
  offline: 'Нет связи с сервером. Проверьте интернет и повторите',
  failed: 'Советчик сейчас недоступен. Попробуйте ещё раз',
  editNoFiles: 'В режиме правки файлы не принимаются. Опишите изменение словами',
} as const;
