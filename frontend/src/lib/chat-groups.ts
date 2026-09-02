// The Discord-style message grouping of the legacy Offers/Chat.vue:
// consecutive messages of one sender collapse under one avatar+name
// header until the sender changes or more than two minutes pass.
import type { OfferMessage } from './types-offers';

/** Minutes of silence after which a new group starts, like the legacy
 * differenceInMinutes(...) > 2 check. */
export const GROUP_GAP_MINUTES = 2;

export interface MessageGroup {
  sender: OfferMessage['sender'];
  mine: boolean;
  /** The legacy getTimeString of the group's first message. */
  time: string;
  messages: OfferMessage[];
}

/** The legacy getTimeString: Today/Yesterday at HH:mm, else the date. */
export function chatTimestamp(timestampMs: number, nowMs: number = Date.now()): string {
  const date = new Date(timestampMs);
  const now = new Date(nowMs);
  const time = `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;

  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate();

  if (sameDay(date, now)) return `Today at ${time}`;
  const yesterday = new Date(nowMs - 24 * 60 * 60 * 1000);
  if (sameDay(date, yesterday)) return `Yesterday at ${time}`;

  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${time}`;
}

/**
 * Parses one message's created_at (ISO UTC from the API) to unix
 * milliseconds.
 */
export function messageTimeMs(message: OfferMessage): number {
  const parsed = Date.parse(message.created_at);
  return Number.isNaN(parsed) ? 0 : parsed;
}

export function groupMessages(
  messages: OfferMessage[],
  nowMs: number = Date.now(),
): MessageGroup[] {
  const groups: MessageGroup[] = [];
  for (const message of messages) {
    const current = groups.at(-1);
    const previous = current?.messages.at(-1);
    const gapMinutes =
      previous === undefined
        ? 0
        : Math.abs(messageTimeMs(message) - messageTimeMs(previous)) / 60_000;

    if (
      current === undefined ||
      previous === undefined ||
      current.sender.id !== message.sender.id ||
      gapMinutes > GROUP_GAP_MINUTES
    ) {
      groups.push({
        sender: message.sender,
        mine: message.mine,
        time: chatTimestamp(messageTimeMs(message), nowMs),
        messages: [message],
      });
    } else {
      current.messages.push(message);
    }
  }
  return groups;
}
