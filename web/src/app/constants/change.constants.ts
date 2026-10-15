import { ChangeState } from '../models';

export const CHANGE_FIRST_POLL_DELAY_MS = 4000;

export const CHANGE_ACTIVE_POLL_INTERVAL_MS = 10_000;

export const CHANGE_REVIEW_POLL_INTERVAL_MS = 30_000;

export const CHANGE_ACTIVE_LIMIT_MS = 45 * 60 * 1000;

export const CHANGE_REVIEW_LIMIT_MS = 6 * 60 * 60 * 1000;

export const CHANGE_ACTIVE_STATES: readonly ChangeState[] = ['queued', 'working'];

export const CHANGE_TRACKED_STATES: readonly ChangeState[] = [
  'queued',
  'working',
  'ready',
  'draft',
];

export const CHANGE_STATE_TEXT = {
  queued: 'Заявка принята и ждёт очереди…',
  working: 'Claude работает над изменением. Обычно это 1–5 минут…',
  ready:
    'Изменение готово. Откройте его, проверьте и нажмите Merge: тогда оно попадёт в приложение.',
  draft:
    'Изменение подготовлено, но тесты не прошли. Откройте и посмотрите, принимать его не стоит.',
  merged: 'Изменение принято. Приложение обновится за пару минут, перезапустите его.',
  closed: 'Заявка закрыта без принятия: изменение в приложение не попало.',
  failed: 'Не получилось: запуск завершился ошибкой. Откройте журнал запуска.',
  nochange:
    'Claude ничего не изменил: запрос неясен или правка не нужна. Уточните и отправьте снова.',
  unknown: 'Статус заявки неизвестен. Проверьте список заявок на GitHub.',
} as const satisfies Record<ChangeState, string>;

export const CHANGE_STATES = Object.keys(CHANGE_STATE_TEXT) as ChangeState[];

export const CHANGE_LIST_LINK_LABEL = 'Открыть заявки на GitHub';

export const CHANGE_RUN_LINK_LABEL = 'Открыть журнал запуска';

export const CHANGE_BADGE_TEXT = 'Правка приложения';
