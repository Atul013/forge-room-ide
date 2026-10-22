'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { confirmThemePreview } = require('../themePreview');

test('a theme is previewed before a Keep choice commits it', async () => {
	const events = [];
	const kept = await confirmThemePreview(
		async () => events.push('preview'),
		async () => { events.push('confirm'); return true; },
		async () => events.push('restore')
	);
	assert.equal(kept, true);
	assert.deepEqual(events, ['preview', 'confirm']);
});

test('Restore previous and dismissing the confirmation both restore the original appearance', async () => {
	for (const response of [false, undefined]) {
		const events = [];
		const kept = await confirmThemePreview(
			async () => events.push('preview'),
			async () => { events.push('confirm'); return response; },
			async () => events.push('restore')
		);
		assert.equal(kept, false);
		assert.deepEqual(events, ['preview', 'confirm', 'restore']);
	}
});

test('a failed confirmation restores the prior appearance before returning the error', async () => {
	const events = [];
	await assert.rejects(() => confirmThemePreview(
		async () => events.push('preview'),
		async () => { events.push('confirm'); throw new Error('dialog failed'); },
		async () => events.push('restore')
	), /dialog failed/);
	assert.deepEqual(events, ['preview', 'confirm', 'restore']);
});

test('a partial preview failure restores settings changed before the failure', async () => {
	const events = [];
	await assert.rejects(() => confirmThemePreview(
		async () => { events.push('change-theme'); events.push('change-colors'); throw new Error('setting rejected'); },
		async () => { events.push('confirm'); return true; },
		async () => events.push('restore-all')
	), /setting rejected/);
	assert.deepEqual(events, ['change-theme', 'change-colors', 'restore-all']);
});
