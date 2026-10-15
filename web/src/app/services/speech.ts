import { Service, signal } from '@angular/core';
import { SPEECH_LANG } from '../constants';

@Service()
export class Speech {
  readonly speakingId = signal<string | undefined>(undefined);
  readonly synthesisSupported = typeof window !== 'undefined' && 'speechSynthesis' in window;

  speak(id: string, text: string): void {
    if (!this.synthesisSupported) {
      return;
    }
    this.stopSpeaking();

    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = SPEECH_LANG;
    const voice = speechSynthesis
      .getVoices()
      .find((candidate) => candidate.lang.toLowerCase().startsWith('ru'));
    if (voice) {
      utterance.voice = voice;
    }
    utterance.onend = () => this.clearSpeaking(id);
    utterance.onerror = () => this.clearSpeaking(id);

    this.speakingId.set(id);
    speechSynthesis.speak(utterance);
  }

  stopSpeaking(): void {
    if (!this.synthesisSupported) {
      return;
    }
    speechSynthesis.cancel();
    this.speakingId.set(undefined);
  }

  private clearSpeaking(id: string): void {
    if (this.speakingId() === id) {
      this.speakingId.set(undefined);
    }
  }
}
