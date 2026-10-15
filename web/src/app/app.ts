import {
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  afterRenderEffect,
  effect,
  inject,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { AccessGate } from './components/access-gate/access-gate';
import { Composer } from './components/composer/composer';
import { MessageBubble } from './components/message-bubble/message-bubble';
import { SettingsPanel } from './components/settings-panel/settings-panel';
import {
  CLEAR_CONFIRM_TIMEOUT_MS,
  COMPOSER_PLACEHOLDERS,
  EDIT_MODE_HINT,
  SETTINGS_OPEN_LABEL,
  UPDATE_BUTTON_LABEL,
} from './constants';
import { ComposerPayload } from './models';
import { Access } from './services/access';
import { AppUpdate } from './services/app-update';
import { Chat } from './services/chat';
import { Settings } from './services/settings';
import { Speech } from './services/speech';
import { APP_VERSION } from './version';

@Component({
  imports: [AccessGate, Composer, MessageBubble, SettingsPanel],
  selector: 'app-root',
  styleUrl: './app.scss',
  templateUrl: './app.html',
})
export class App {
  protected readonly access = inject(Access);
  protected readonly chat = inject(Chat);
  protected readonly speech = inject(Speech);
  protected readonly settings = inject(Settings);
  protected readonly appUpdate = inject(AppUpdate);
  protected readonly settingsOpenLabel = SETTINGS_OPEN_LABEL;
  protected readonly updateLabel = UPDATE_BUTTON_LABEL;
  protected readonly version = APP_VERSION;
  protected readonly placeholders = COMPOSER_PLACEHOLDERS;
  protected readonly editHint = EDIT_MODE_HINT;
  protected readonly composing = signal(false);
  protected readonly confirmingClear = signal(false);
  protected readonly settingsOpen = signal(false);

  private readonly destroyRef = inject(DestroyRef);
  private readonly log = viewChild.required<ElementRef<HTMLElement>>('log');
  private stickToBottom = true;

  constructor() {
    effect(() => {
      const reply = this.chat.lastReply();
      if (reply && untracked(() => this.settings.speakReplies())) {
        untracked(() => this.speech.speak(reply.id, reply.text));
      }
    });

    afterRenderEffect(() => {
      this.chat.messages();
      this.stickToBottom = true;
      this.scrollToBottom('smooth');
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
