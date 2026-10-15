import { DatePipe } from '@angular/common';
import { Component, computed, input, output } from '@angular/core';
import { CHANGE_ACTIVE_STATES, CHANGE_BADGE_TEXT, CHANGE_LIST_LINK_LABEL } from '../../constants';
import { ChatMessage } from '../../models';

@Component({
  imports: [DatePipe],
  selector: 'app-message-bubble',
  styleUrl: './message-bubble.scss',
  templateUrl: './message-bubble.html',
})
export class MessageBubble {
  readonly message = input.required<ChatMessage>();
  readonly speaking = input(false);
  readonly canSpeak = input(false);
  readonly speakRequested = output();
  readonly stopRequested = output();
  readonly deleteRequested = output();

  protected readonly badgeText = CHANGE_BADGE_TEXT;
  protected readonly defaultLinkLabel = CHANGE_LIST_LINK_LABEL;
  protected readonly fromUser = computed(() => this.message().role === 'user');
  protected readonly working = computed(() => {
    const state = this.message().changeState;
    return state !== undefined && CHANGE_ACTIVE_STATES.includes(state);
  });
  protected readonly speakable = computed(
    () =>
      this.canSpeak() &&
      !this.fromUser() &&
      !this.message().pending &&
      this.message().text.length > 0,
  );
}
