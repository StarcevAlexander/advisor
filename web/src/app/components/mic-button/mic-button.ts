import { Component, computed, input, output } from '@angular/core';
import { MIC_LABELS } from '../../constants';
import { RecognitionState } from '../../models';

@Component({
  imports: [],
  selector: 'app-mic-button',
  styleUrl: './mic-button.scss',
  templateUrl: './mic-button.html',
})
export class MicButton {
  readonly state = input<RecognitionState>('idle');
  readonly compact = input(false);
  readonly pressed = output();

  protected readonly listening = computed(() => this.state() === 'listening');
  protected readonly label = computed(() => MIC_LABELS[this.state()]);
  protected readonly problem = computed(
    () => this.state() !== 'idle' && this.state() !== 'listening',
  );
}
