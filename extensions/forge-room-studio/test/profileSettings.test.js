'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { createProfileSnapshot, mergeProfileSettings } = require('../profileSettings');

test('each preview snapshots exactly the settings it changes at that scope', () => {
	assert.deepEqual(createProfileSnapshot(['workbench.colorTheme', 'editor.fontSize'], {
		'workbench.colorTheme': 'Forge Room · Neon Ronin'
	}, 'workspace'), {
		settings: {
			'workbench.colorTheme': { target: 'workspace', value: 'Forge Room · Neon Ronin' },
			'editor.fontSize': { target: 'workspace', value: null }
		}
	});
});

test('profile colors override their selected keys and preserve unrelated colors', () => {
	const imported = { 'workbench.colorCustomizations': { 'editor.background': '#111111' } };
	const current = { 'workbench.colorCustomizations': { 'editor.background': '#222222', 'terminal.background': '#333333' } };
	assert.deepEqual(mergeProfileSettings(imported, current), {
		'workbench.colorCustomizations': { 'editor.background': '#111111', 'terminal.background': '#333333' }
	});
	assert.deepEqual(imported['workbench.colorCustomizations'], { 'editor.background': '#111111' });
});

test('profile syntax colors preserve unrelated scopes and merge foreground into a matching rule', () => {
	const imported = { 'editor.tokenColorCustomizations': { textMateRules: [
		{ scope: 'comment', settings: { foreground: '#111111' } },
		{ scope: 'keyword', settings: { foreground: '#444444' } }
	] } };
	const current = { 'editor.tokenColorCustomizations': {
		semanticHighlighting: false,
		textMateRules: [
			{ scope: 'comment', settings: { foreground: '#222222', fontStyle: 'italic' } },
			{ scope: 'entity.name.class', settings: { foreground: '#333333' } }
		]
	} };
	assert.deepEqual(mergeProfileSettings(imported, current)['editor.tokenColorCustomizations'], {
		semanticHighlighting: false,
		textMateRules: [
			{ scope: 'comment', settings: { foreground: '#111111', fontStyle: 'italic' } },
			{ scope: 'entity.name.class', settings: { foreground: '#333333' } },
			{ scope: 'keyword', settings: { foreground: '#444444' } }
		]
	});
});
