import { Component, computed, inject, input, model, output, signal } from '@angular/core';
import { form, FormField } from '@angular/forms/signals';
import { ACCEPTED_FILE_TYPES, MAX_ATTACHMENT_SIZE_BYTES, MAX_ATTACHMENTS } from '../../constants';
import { Attachment, ComposerPayload } from '../../models';
import { ImageCompressor } from '../../services/image-compressor';

@Component({
  imports: [FormField],
  selector: 'app-composer',
  styleUrl: './composer.scss',
  templateUrl: './composer.html',
})
export class Composer {
  readonly busy = input(false);
  readonly expanded = model(false);
  readonly submitted = output<ComposerPayload>();

  private readonly imageCompressor = inject(ImageCompressor);

  protected readonly acceptedTypes = ACCEPTED_FILE_TYPES;
  protected readonly model = signal({ text: '' });
  protected readonly composerForm = form(this.model);
  protected readonly attachments = signal<Attachment[]>([]);
  protected readonly error = signal<string | undefined>(undefined);
  protected readonly canSend = computed(
    () =>
      !this.busy() &&
      (this.composerForm.text().value().trim().length > 0 || this.attachments().length > 0),
  );

  protected async onFilesSelected(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    input.value = '';
    this.error.set(undefined);

    const accepted: Attachment[] = [];
    for (const file of files) {
      if (this.attachments().length + accepted.length >= MAX_ATTACHMENTS) {
        this.error.set(`Можно приложить не больше ${MAX_ATTACHMENTS} файлов`);
        break;
      }
      if (file.size > MAX_ATTACHMENT_SIZE_BYTES) {
        this.error.set(`Файл «${file.name}» больше 20 МБ`);
        continue;
      }
      const prepared = await this.imageCompressor.prepare(file);
      accepted.push({
        id: crypto.randomUUID(),
        name: prepared.name,
        mimeType: prepared.type,
        size: prepared.size,
        file: prepared,
        previewUrl: prepared.type.startsWith('image/') ? URL.createObjectURL(prepared) : undefined,
      });
    }
    this.attachments.update((current) => [...current, ...accepted]);
  }

  protected removeAttachment(id: string): void {
    const target = this.attachments().find((attachment) => attachment.id === id);
    if (target?.previewUrl) {
      URL.revokeObjectURL(target.previewUrl);
    }
    this.attachments.update((current) => current.filter((attachment) => attachment.id !== id));
    this.error.set(undefined);
  }

  protected onFocusIn(): void {
    this.expanded.set(true);
  }

  protected onFocusOut(event: FocusEvent): void {
    const next = event.relatedTarget as Node | null;
    if (next && (event.currentTarget as HTMLElement).contains(next)) {
      return;
    }
    if (this.composerForm.text().value().trim().length === 0 && this.attachments().length === 0) {
      this.expanded.set(false);
    }
  }

  protected onEscape(event: Event): void {
    (event.target as HTMLElement).blur();
  }

  protected onEnter(event: Event): void {
    if ((event as KeyboardEvent).shiftKey) {
      return;
    }
    event.preventDefault();
    this.send();
  }

  protected send(): void {
    if (!this.canSend()) {
      return;
    }
    this.submitted.emit({
      text: this.composerForm.text().value().trim(),
      attachments: this.attachments(),
    });
    this.model.set({ text: '' });
    this.attachments.set([]);
    this.error.set(undefined);
  }
}
