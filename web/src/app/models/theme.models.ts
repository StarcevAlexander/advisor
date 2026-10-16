export type ThemeId =
  'light' | 'dark' | 'ocean' | 'forest' | 'purple' | 'sakura' | 'sunset' | 'contrast';

export interface ThemeOption {
  id: ThemeId;
  label: string;
  themeColor: string;
}

export type SettingsScreen = 'main' | 'appearance' | 'theme' | 'scale';
