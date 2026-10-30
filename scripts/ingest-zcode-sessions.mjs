import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';

const SOURCE = 'zcode';
const BATCH_SIZE = 200;

const ENV_FILE = process.env.ADVISOR_ENV_FILE ?? join(homedir(), '.claude', 'advisor-ingest.env');
const DEFAULT_DB = join(homedir(), '.zcode', 'cli', 'db', 'db.sqlite');

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
const dryRun = args.includes('--dry-run');
const allProjects = args.includes('--all');
const dbFlagIndex = args.indexOf('--db');
const dbPath = resolve(dbFlagIndex >= 0 ? args[dbFlagIndex + 1] : DEFAULT_DB);
const target = args.find((arg, index) => !arg.startsWith('--') && (index === 0 || args[index - 1] !== '--db'));

if (!dryRun && (!url || !password)) {
  console.error(`Нужны ADVISOR_URL и ADVISOR_PASSWORD (переменные окружения или файл ${ENV_FILE}); для проверки без отправки есть --dry-run`);
  process.exit(2);
}

let DatabaseSync;
try {
  ({ DatabaseSync } = await import('node:sqlite'));
} catch {
  console.error('Нужен Node.js 22.5+ со встроенным модулем node:sqlite');
  process.exit(2);
}

const projectDir = resolve(target ?? process.cwd());
const db = new DatabaseSync(dbPath, { readOnly: true });

function sessions() {
  if (allProjects) {
    return db.prepare('select id, directory from session order by time_created').all();
  }
  return db
    .prepare('select id, directory from session where lower(directory) = lower(?) order by time_created')
    .all(projectDir);
}

function rowsFor(sessionId) {
  return db
    .prepare(
      `select m.id as message_id, m.sequence as message_sequence, m.data as message_data, p.sequence as part_sequence, p.data as part_data
       from message m left join part p on p.message_id = m.id
       where m.session_id = ?
       order by m.sequence, p.sequence`,
    )
    .all(sessionId);
}

function messageText(row) {
  try {
    const part = JSON.parse(row.part_data ?? '{}');
    if (part.type !== 'text' || part.synthetic) {
      return '';
    }
    return typeof part.text === 'string' ? part.text : '';
  } catch {
    return '';
  }
}

function toMessages(rows) {
  const byMessage = new Map();
  for (const row of rows) {
    let entry = byMessage.get(row.message_id);
    if (!entry) {
      let data;
      try {
        data = JSON.parse(row.message_data);
      } catch {
        continue;
      }
      entry = { role: data.role, origin: data.semantics?.origin, synthetic: Boolean(data.synthetic), created: data.time?.created, text: '' };
      byMessage.set(row.message_id, entry);
    }
    entry.text += (entry.text ? '\n' : '') + messageText(row);
  }
  const messages = [];
  for (const [id, entry] of byMessage) {
    const text = entry.text.trim();
    if (!text || entry.synthetic || (entry.role === 'user' && entry.origin !== 'real_user') || (entry.role !== 'user' && entry.role !== 'assistant')) {
      continue;
    }
    messages.push({
      externalId: id,
      role: entry.role,
      text,
      at: new Date(entry.created ?? 0).toISOString(),
    });
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

try {
  const found = sessions();
  if (found.length === 0) {
    console.log(`Сессий ZCode для ${allProjects ? 'всех проектов' : projectDir} не найдено`);
  }
  for (const session of found) {
    const messages = toMessages(rowsFor(session.id));
    if (messages.length === 0) {
      continue;
    }
    if (!dryRun) {
      for (let i = 0; i < messages.length; i += BATCH_SIZE) {
        await post(session.id, messages.slice(i, i + BATCH_SIZE));
      }
    }
    console.log(`${session.id}: ${messages.length}${dryRun ? ' (dry run)' : ''}`);
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
} finally {
  db.close();
}
