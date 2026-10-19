import { readFileSync } from 'node:fs';
import { readdir, readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { basename, dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const SOURCE = 'claude-code';
const BATCH_SIZE = 200;
const REMINDER_BLOCK = /<system-reminder>[\s\S]*?<\/system-reminder>/g;

const ENV_FILE = process.env.ADVISOR_ENV_FILE ?? join(homedir(), '.claude', 'advisor-ingest.env');
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function readEnvFile(path) {
  try {
    return Object.fromEntries(
      readFileSync(path, 'utf8')
        .split('\n')
        .map((line) => line.trim())
        .filter((line) => line && !line.startsWith('#') && line.includes('='))
        .map((line) => {
          const index = line.indexOf('=');
          return [line.slice(0, index).trim(), line.slice(index + 1).trim()];
        }),
    );
  } catch {
    return {};
  }
}

const fileEnv = readEnvFile(ENV_FILE);
const url = (process.env.ADVISOR_URL ?? fileEnv.ADVISOR_URL ?? '').replace(/\/+$/, '');
const password = process.env.ADVISOR_PASSWORD ?? fileEnv.ADVISOR_PASSWORD ?? '';
const args = process.argv.slice(2);
const fromHook = args.includes('--hook');
const target = args.find((arg) => !arg.startsWith('--'));

if (!url || !password) {
  console.error(`Нужны ADVISOR_URL и ADVISOR_PASSWORD (переменные окружения или файл ${ENV_FILE})`);
  process.exit(fromHook ? 0 : 2);
}

function projectSlug(cwd) {
  return resolve(cwd).replace(/[:\\/]/g, '-');
}

function textOf(content) {
  if (typeof content === 'string') {
    return content;
  }
  if (!Array.isArray(content)) {
    return '';
  }
  return content
    .filter((block) => block.type === 'text' && typeof block.text === 'string')
    .map((block) => block.text)
    .join('\n');
}

function toMessage(entry) {
  if ((entry.type !== 'user' && entry.type !== 'assistant') || entry.isSidechain || !entry.uuid) {
    return undefined;
  }
  const text = textOf(entry.message?.content).replace(REMINDER_BLOCK, '').trim();
  if (!text) {
    return undefined;
  }
  return {
    externalId: entry.uuid,
    role: entry.type,
    text,
    at: entry.timestamp,
  };
}

async function readTranscript(path) {
  const raw = await readFile(path, 'utf8');
  const messages = [];
  for (const line of raw.split('\n')) {
    if (!line.trim()) {
      continue;
    }
    try {
      const message = toMessage(JSON.parse(line));
      if (message) {
        messages.push(message);
      }
    } catch {
      continue;
    }
  }
  return messages;
}

async function post(conversationId, messages) {
  const response = await fetch(`${url}/api/ingest`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${password}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ source: SOURCE, conversationId, messages }),
  });
  if (!response.ok) {
    throw new Error(`${response.status} ${await response.text()}`);
  }
}

async function ingest(path) {
  const conversationId = basename(path, '.jsonl');
  const messages = await readTranscript(path);
  for (let i = 0; i < messages.length; i += BATCH_SIZE) {
    await post(conversationId, messages.slice(i, i + BATCH_SIZE));
  }
  console.log(`${conversationId}: ${messages.length}`);
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString('utf8');
}

async function transcriptPaths() {
  if (fromHook) {
    const payload = JSON.parse(await readStdin());
    const cwd = resolve(payload.cwd ?? '');
    const inRepo = cwd === REPO_ROOT || cwd.startsWith(REPO_ROOT + sep);
    return inRepo && payload.transcript_path ? [payload.transcript_path] : [];
  }
  if (target?.endsWith('.jsonl')) {
    return [target];
  }
  const dir = target ?? join(homedir(), '.claude', 'projects', projectSlug(process.cwd()));
  const names = await readdir(dir);
  return names.filter((name) => name.endsWith('.jsonl')).map((name) => join(dir, name));
}

try {
  for (const path of await transcriptPaths()) {
    await ingest(path);
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(fromHook ? 0 : 1);
}
