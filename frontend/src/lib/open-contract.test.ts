import { describe, expect, it } from 'vitest';
import { openContractFailure } from './open-contract';

describe('openContractFailure', () => {
	it('surfaces the backend message and grant url of the 403 scope failure', () => {
		expect(
			openContractFailure({
				message: 'You need to grant the "Open Window" ESI scope to open th contract ingame!',
				grant_scope_url: 'https://login.eveonline.com/v2/oauth/authorize?scope=esi-ui.open_window.v1'
			})
		).toEqual({
			message: 'You need to grant the "Open Window" ESI scope to open th contract ingame!',
			grantScopeUrl: 'https://login.eveonline.com/v2/oauth/authorize?scope=esi-ui.open_window.v1'
		});
	});

	it('surfaces a plain message without a grant url', () => {
		expect(openContractFailure({ message: 'The contract window could not be opened.' })).toEqual({
			message: 'The contract window could not be opened.',
			grantScopeUrl: null
		});
	});

	it('falls back on an unreadable body', () => {
		for (const body of [null, undefined, 'oops', { message: '' }, { grant_scope_url: 42 }]) {
			expect(openContractFailure(body).grantScopeUrl).toBeNull();
			expect(openContractFailure(body).message).toBe('The contract could not be opened in game.');
		}
	});
});
