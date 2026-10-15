import { Component, ElementRef, effect, inject, model, viewChild } from '@angular/core';
import {
  DEVELOPER_MODE_LABEL,
  SETTINGS_CLOSE_LABEL,
  SETTINGS_TITLE,
  SPEAK_REPLIES_LABEL,
  THEME_SECTION_TITLE,
  THEMES,
} from '../../constants';
import { ThemeId } from '../../models';
import { Settings } from '../../services/settings';
import { Speech } from '../../services/speech';

@Component({
  selector: 'app-settings-panel',
  styleUrl: './settings-panel.scss',
  templateUrl: './settings-panel.html',
})
export class SettingsPanel {
  protected readonly settings = inject(Settings);
  protected readonly speech = inject(Speech);
  protected readonly title = SETTINGS_TITLE;
  protected readonly closeLabel = SETTINGS_CLOSE_LABEL;
  protected readonly speakRepliesLabel = SPEAK_REPLIES_LABEL;
  protected readonly developerModeLabel = DEVELOPER_MODE_LABEL;

  protected readonly themeTitle = THEME_SECTION_TITLE;
  protected readonly themes = THEMES;

  readonly open = model(false);

  private readonly dialog = viewChild.required<ElementRef<HTMLDialogElement>>('dialog');

  constructor() {
    effect(() => {
      const element = this.dialog().nativeElement;
      if (this.open() && !element.open) {
        element.showModal();
      } else if (!this.open() && element.open) {
        element.close();
      }
    });
  }

  protected onSpeakRepliesChanged(event: Event): void {
    const enabled = (event.target as HTMLInputElement).checked;
    this.settings.setSpeakReplies(enabled);
    if (!enabled) {
      this.speech.stopSpeaking();
    }
  }

  protected onDeveloperModeChanged(event: Event): void {
    this.settings.setDeveloperMode((event.target as HTMLInputElement).checked);
  }

  protected onThemeChanged(id: ThemeId): void {
    this.settings.setTheme(id);
  }

  protected onDialogClicked(event: MouseEvent): void {
    if (event.target === this.dialog().nativeElement) {
      this.open.set(false);
    }
  }
}
