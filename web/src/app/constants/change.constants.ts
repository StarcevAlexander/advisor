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
  plan: 'Вот мой план. Отметьте, что делать, и отправьте.',
  cancelled: 'Заявка отменена: ничего не отправлено.',
  queued: 'Заявка принята и ждёт очереди…',
  working: 'ИИ работает над изменением. Обычно это 1–5 минут…',
  ready: 'Изменение готово, но само не принялось. Откройте его, проверьте и нажмите Merge.',
  draft:
    'Изменение подготовлено, но тесты не прошли. Откройте и посмотрите, принимать его не стоит.',
  merged: 'Изменение принято и публикуется. Через пару минут в шапке появится кнопка «Обновить».',
  closed: 'Заявка закрыта без принятия: изменение в приложение не попало.',
  failed: 'Не получилось: запуск завершился ошибкой. Откройте журнал запуска.',
  nochange:
    'ИИ ничего не изменил: запрос неясен или правка не нужна. Уточните и отправьте снова.',
  unknown: 'Статус заявки неизвестен. Проверьте список заявок на GitHub.',
} as const satisfies Record<ChangeState, string>;

export const ROLLBACK_STATE_TEXT = {
  queued: 'Откат принят и ждёт очереди…',
  working: 'Откат выполняется. Обычно это 1–3 минуты…',
  ready: 'Откат готов, но сам не принялся. Откройте его на GitHub и нажмите Merge.',
  draft:
    'Откат подготовлен, но тесты после него не прошли. Откройте и посмотрите, принимать его не стоит.',
  merged:
    'Последняя правка откачена и публикуется. Через пару минут в шапке появится кнопка «Обновить».',
  closed: 'Откат закрыт без принятия: приложение не изменилось.',
  failed:
    'Откат не удался: чаще всего позже были другие правки, которые зависят от этой. Откройте журнал запуска.',
  nochange: 'Откатывать нечего: принятых правок ИИ не осталось.',
  unknown: 'Статус отката неизвестен. Проверьте список заявок на GitHub.',
} as const satisfies Record<Exclude<ChangeState, 'plan' | 'cancelled'>, string>;

export const CHANGE_STATES = Object.keys(CHANGE_STATE_TEXT) as ChangeState[];

export const ROLLBACK_MENU_LABEL = 'Откатить последнюю правку';

export const ROLLBACK_MENU_HINT =
  'Вернёт приложение к виду до последней правки, которую сделал ИИ.';

export const ROLLBACK_CONFIRM_TITLE = 'Откатить последнюю правку?';

export const ROLLBACK_CONFIRM_TEXT =
  'ИИ вернёт приложение к состоянию до последней принятой правки и выложит результат, это займёт пару минут. Если после той правки менялись те же места, откат может не получиться: тогда ничего не изменится.';

export const ROLLBACK_CONFIRM_LABEL = 'Откатить';

export const CHANGE_LIST_LINK_LABEL = 'Открыть заявки на GitHub';

export const CHANGE_RUN_LINK_LABEL = 'Открыть журнал запуска';

export const CHANGE_BADGE_TEXT = 'Правка приложения';

export const CHANGE_CONFIRM_LABEL = 'Отправить выбранное';

export const CHANGE_REQUEST_MAX_RUNES = 2000;

export const CHANGE_REQUEST_STEPS_HEADER = 'Сделай только эти изменения:';

export const CHANGE_PLAN_STEPS_HEADER = 'Предложенные пункты:';

export const UPDATE_BUTTON_LABEL = 'Обновить';

export const CHANGE_CANCEL_LABEL = 'Отмена';
