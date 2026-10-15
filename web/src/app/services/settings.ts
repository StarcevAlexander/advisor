import { Service, effect, signal } from '@angular/core';
import {
  DEFAULT_THEME,
  DEVELOPER_MODE_STORAGE_KEY,
  SPEAK_REPLIES_STORAGE_KEY,
  THEME_STORAGE_KEY,
  THEMES,
} from '../constants';
import { ThemeId } from '../models';

@Service()
export class Settings {
  readonly speakReplies = signal(this.restoreFlag(SPEAK_REPLIES_STORAGE_KEY));
  readonly developerMode = signal(this.restoreFlag(DEVELOPER_MODE_STORAGE_KEY));

  readonly theme = signal<ThemeId>(this.restoreTheme());

  constructor() {
    effect(() => {
      const id = this.theme();
      document.documentElement.dataset['theme'] = id;
      const color = THEMES.find((option) => option.id === id)?.themeColor;
      if (color) {
        document.querySelector('meta[name="theme-color"]')?.setAttribute('content', color);
      }
    });
  }

  setTheme(id: ThemeId): void {
    this.theme.set(id);
    try {
      localStorage.setItem(THEME_STORAGE_KEY, id);
    } catch {
      return;
    }
  }

  setSpeakReplies(enabled: boolean): void {
    this.speakReplies.set(enabled);
    this.storeFlag(SPEAK_REPLIES_STORAGE_KEY, enabled);
  }

  setDeveloperMode(enabled: boolean): void {
    this.developerMode.set(enabled);
    this.storeFlag(DEVELOPER_MODE_STORAGE_KEY, enabled);
  }

  private storeFlag(key: string, enabled: boolean): void {
    try {
      localStorage.setItem(key, JSON.stringify(enabled));
    } catch {
      return;
    }
  }

  private restoreFlag(key: string): boolean {
    try {
      return localStorage.getItem(key) === 'true';
    } catch {
      return false;
    }
  }

  private restoreTheme(): ThemeId {
    try {
      const stored = localStorage.getItem(THEME_STORAGE_KEY);
      return THEMES.find((option) => option.id === stored)?.id ?? DEFAULT_THEME;
    } catch {
      return DEFAULT_THEME;
    }
  }
}
