import { Service, signal } from '@angular/core';
import { RECOGNITION_LANG, SPEECH_LANG } from '../constants';
import {
  RecognitionState,
  SpeechRecognitionConstructor,
  SpeechRecognitionLike,
  SpeechRecognitionWindow,
} from '../models';

@Service()
export class Speech {
  readonly recognitionState = signal<RecognitionState>('idle');
  readonly interimText = signal('');
  readonly speakingId = signal<string | undefined>(undefined);
  readonly synthesisSupported = typeof window !== 'undefined' && 'speechSynthesis' in window;

  private recognition?: SpeechRecognitionLike;
  private finalText = '';
  private onFinal?: (text: string) => void;

  listen(onFinal: (text: string) => void): void {
    const recognitionConstructor = this.resolveConstructor();
    if (!recognitionConstructor) {
      this.recognitionState.set('unsupported');
      return;
    }

    this.stopSpeaking();
    this.finalText = '';
    this.interimText.set('');
    this.onFinal = onFinal;

    const recognition = new recognitionConstructor();
    recognition.lang = RECOGNITION_LANG;
    recognition.continuous = false;
    recognition.interimResults = true;

    recognition.onresult = (event) => {
      let interim = '';
      for (let index = event.resultIndex; index < event.results.length; index++) {
        const result = event.results[index];
        if (result.isFinal) {
          this.finalText += result[0].transcript;
        } else {
          interim += result[0].transcript;
        }
      }
      this.interimText.set((this.finalText + interim).trim());
    };

    recognition.onerror = (event) => {
      if (event.error === 'not-allowed' || event.error === 'service-not-allowed') {
        this.recognitionState.set('denied');
      } else if (event.error !== 'aborted') {
        this.recognitionState.set('failed');
      }
    };

    recognition.onend = () => this.finish();

    this.recognition = recognition;
    this.recognitionState.set('listening');
    recognition.start();
  }

  stopListening(): void {
    this.recognition?.stop();
  }

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

  resetRecognitionError(): void {
    if (this.recognitionState() !== 'listening' && this.recognitionState() !== 'unsupported') {
      this.recognitionState.set('idle');
    }
  }

  private finish(): void {
    const text = (this.finalText || this.interimText()).trim();
    if (this.recognitionState() === 'listening') {
      this.recognitionState.set('idle');
    }
    this.interimText.set('');
    this.recognition = undefined;
    if (text) {
      this.onFinal?.(text);
    }
  }

  private clearSpeaking(id: string): void {
    if (this.speakingId() === id) {
      this.speakingId.set(undefined);
    }
  }

  private resolveConstructor(): SpeechRecognitionConstructor | undefined {
    if (typeof window === 'undefined') {
      return undefined;
    }
    const speechWindow = window as unknown as SpeechRecognitionWindow;
    return speechWindow.SpeechRecognition ?? speechWindow.webkitSpeechRecognition;
  }
}
