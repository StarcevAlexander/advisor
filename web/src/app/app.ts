import {
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  afterRenderEffect,
  effect,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { AccessGate } from './components/access-gate/access-gate';
import { Composer } from './components/composer/composer';
import { MessageBubble } from './components/message-bubble/message-bubble';
import { MicButton } from './components/mic-button/mic-button';
import { CLEAR_CONFIRM_TIMEOUT_MS, COMPOSER_PLACEHOLDERS, EDIT_MODE_HINT } from './constants';
import { ComposerPayload } from './models';
import { Access } from './services/access';
import { AppUpdate } from './services/app-update';
import { Chat } from './services/chat';
import { Speech } from './services/speech';
import { APP_VERSION } from './version';

@Component({
  imports: [AccessGate, Composer, MessageBubble, MicButton],
  selector: 'app-root',
  styleUrl: './app.scss',
  templateUrl: './app.html',
})
export class App {
  protected readonly access = inject(Access);
  protected readonly chat = inject(Chat);
  protected readonly speech = inject(Speech);
  private readonly appUpdate = inject(AppUpdate);
  protected readonly version = APP_VERSION;
  protected readonly placeholders = COMPOSER_PLACEHOLDERS;
  protected readonly editHint = EDIT_MODE_HINT;
  protected readonly composing = signal(false);
  protected readonly confirmingClear = signal(false);

  private readonly destroyRef = inject(DestroyRef);
  private readonly log = viewChild.required<ElementRef<HTMLElement>>('log');
  private stickToBottom = true;

  constructor() {
    afterRenderEffect(() => {
      this.chat.messages();
      this.stickToBottom = true;
      this.scrollToBottom('smooth');
    });

    effect(() => {
      const idle =
        !this.composing() && !this.chat.busy() && this.speech.recognitionState() !== 'listening';
      if (this.appUpdate.ready() && idle) {
        void this.appUpdate.apply();
      }
    });

    afterNextRender(() => {
      this.appUpdate.start();

      const viewport = window.visualViewport;
      if (viewport) {
        const syncViewport = () => {
          document.documentElement.style.setProperty('--app-height', `${viewport.height}px`);
          window.scrollTo(0, 0);
        };
        syncViewport();
        viewport.addEventListener('resize', syncViewport);
        this.destroyRef.onDestroy(() => viewport.removeEventListener('resize', syncViewport));
      }

      const observer = new ResizeObserver(() => {
        if (this.stickToBottom) {
          this.scrollToBottom('auto');
        }
      });
      observer.observe(this.log().nativeElement);
      this.destroyRef.onDestroy(() => observer.disconnect());
    });
  }

  protected onLogScrolled(): void {
    const element = this.log().nativeElement;
    this.stickToBottom = element.scrollHeight - element.scrollTop - element.clientHeight < 48;
  }

  protected onMicPressed(): void {
    if (this.speech.recognitionState() === 'listening') {
      this.speech.stopListening();
      return;
    }
    this.speech.resetRecognitionError();
    this.speech.listen((text) => this.chat.send({ text, attachments: [] }));
  }

  protected onClearClicked(): void {
    if (!this.confirmingClear()) {
      this.confirmingClear.set(true);
      setTimeout(() => this.confirmingClear.set(false), CLEAR_CONFIRM_TIMEOUT_MS);
      return;
    }
    this.speech.stopSpeaking();
    this.chat.clear();
    this.confirmingClear.set(false);
  }

  protected onDeleteRequested(id: string): void {
    if (this.speech.speakingId() === id) {
      this.speech.stopSpeaking();
    }
    this.chat.remove(id);
  }

  protected onSubmitted(payload: ComposerPayload): void {
    this.speech.stopSpeaking();
    this.chat.send(payload);
  }

  private scrollToBottom(behavior: ScrollBehavior): void {
    const element = this.log().nativeElement;
    element.scrollTo({ top: element.scrollHeight, behavior });
  }
}
