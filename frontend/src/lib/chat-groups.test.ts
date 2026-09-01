import { describe, expect, it } from 'vitest';
import { chatTimestamp, groupMessages } from './chat-groups';
import type { OfferMessage } from './types-offers';

const NOW = Date.parse('2026-08-27T18:00:00Z');

function message(
	id: number,
	senderId: number,
	createdAt: string,
	mine = senderId === 1,
): OfferMessage {
	return {
		id,
		sender: { id: senderId, name: `Char ${senderId}` },
		content: `message ${id}`,
		created_at: createdAt,
		mine,
	};
}

describe('groupMessages', () => {
	it('collapses consecutive messages of one sender', () => {
		const groups = groupMessages(
			[
				message(1, 1, '2026-08-27T17:00:00Z'),
				message(2, 1, '2026-08-27T17:01:00Z'),
				message(3, 2, '2026-08-27T17:01:30Z'),
			],
			NOW,
		);
		expect(groups.length).toBe(2);
		expect(groups[0].messages.map((m) => m.id)).toEqual([1, 2]);
		expect(groups[1].sender.id).toBe(2);
	});

	it('starts a new group after the two-minute gap', () => {
		const groups = groupMessages(
			[message(1, 1, '2026-08-27T17:00:00Z'), message(2, 1, '2026-08-27T17:03:00Z')],
			NOW,
		);
		expect(groups.length).toBe(2);
	});
});

describe('chatTimestamp', () => {
	it('renders today, yesterday and older dates like legacy', () => {
		const local = (iso: string) => Date.parse(iso);
		expect(chatTimestamp(local('2026-08-27T15:04:00'), Date.parse('2026-08-27T18:00:00'))).toBe(
			'Today at 15:04',
		);
		expect(chatTimestamp(local('2026-08-26T09:30:00'), Date.parse('2026-08-27T18:00:00'))).toBe(
			'Yesterday at 09:30',
		);
		expect(chatTimestamp(local('2026-08-20T09:30:00'), Date.parse('2026-08-27T18:00:00'))).toBe(
			'2026-08-20 09:30',
		);
	});
});
