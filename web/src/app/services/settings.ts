import { Service, signal } from '@angular/core';
import { DEVELOPER_MODE_STORAGE_KEY, SPEAK_REPLIES_STORAGE_KEY } from '../constants';

@Service()
export class Settings {
  readonly speakReplies = signal(this.restoreFlag(SPEAK_REPLIES_STORAGE_KEY));
  readonly developerMode = signal(this.restoreFlag(DEVELOPER_MODE_STORAGE_KEY));

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
}
