import { Service, signal } from '@angular/core';
import { SPEAK_REPLIES_STORAGE_KEY } from '../constants';

@Service()
export class Settings {
  readonly speakReplies = signal(this.restoreSpeakReplies());

  setSpeakReplies(enabled: boolean): void {
    this.speakReplies.set(enabled);
    try {
      localStorage.setItem(SPEAK_REPLIES_STORAGE_KEY, JSON.stringify(enabled));
    } catch {
      return;
    }
  }

  private restoreSpeakReplies(): boolean {
    try {
      return localStorage.getItem(SPEAK_REPLIES_STORAGE_KEY) === 'true';
    } catch {
      return false;
    }
  }
}
