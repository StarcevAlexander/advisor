export const API_DEV_URL = 'http://localhost:8080';

export const API_PRODUCTION_URL = 'https://advisor-server.onrender.com';

export const API_CHAT_PATH = '/api/chat';

export const HTTP_UNAUTHORIZED = 401;

export const HTTP_OFFLINE = 0;

export const ACCESS_PASSWORD_STORAGE_KEY = 'advisor.access-password';

export const API_MESSAGES = {
  notConfigured: 'Адрес сервера советчика не задан',
  unauthorized: 'Неверный пароль. Введите его заново',
  offline: 'Нет связи с сервером. Проверьте интернет и повторите',
  failed: 'Советчик сейчас недоступен. Попробуйте ещё раз',
} as const;
