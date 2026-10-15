import { ThemeId, ThemeOption } from '../models';

export const THEME_STORAGE_KEY = 'advisor.theme';

export const DEFAULT_THEME: ThemeId = 'light';

export const THEME_SECTION_TITLE = 'Цветовая тема';

export const THEMES: readonly ThemeOption[] = [
  { id: 'light', label: 'Светлая', themeColor: '#dde7ee' },
  { id: 'dark', label: 'Тёмная', themeColor: '#0f141a' },
  { id: 'ocean', label: 'Океан', themeColor: '#cfe6f5' },
  { id: 'forest', label: 'Лес', themeColor: '#d5e8d8' },
  { id: 'purple', label: 'Фиолетовая', themeColor: '#e6dff5' },
  { id: 'sakura', label: 'Сакура', themeColor: '#f8e1ea' },
  { id: 'sunset', label: 'Закат', themeColor: '#fbe5d0' },
  { id: 'contrast', label: 'Контрастная', themeColor: '#000000' },
];
