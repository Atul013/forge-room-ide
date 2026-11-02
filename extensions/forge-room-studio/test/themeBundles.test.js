'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { createThemeBundle, validateThemeBundle } = require('../themeBundles');

test('imports only supported VS Code colors and token scopes', () => {
	const theme = validateThemeBundle({
		format: 'forge-room-theme-bundle', version: 1, name: '  Night Garden  ',
		colors: { 'editor.background': '#101820', 'statusBar.background': '#223344' },
		tokenColors: [{ scope: 'keyword', settings: { foreground: '#AABBCC' } }]
	});
	assert.equal(theme.name, 'Night Garden');
	assert.deepEqual(theme.settings['workbench.colorCustomizations'], { 'editor.background': '#101820', 'statusBar.background': '#223344' });
	assert.deepEqual(theme.settings['editor.tokenColorCustomizations'], { textMateRules: [{ scope: 'keyword', settings: { foreground: '#AABBCC' } }] });
});

test('rejects unknown keys, malformed colors, unsupported scopes, and duplicate rules', () => {
	const base = { format: 'forge-room-theme-bundle', version: 1, name: 'Test' };
	assert.throws(() => validateThemeBundle({ ...base, script: 'run me', colors: { 'editor.background': '#000000' } }));
	assert.throws(() => validateThemeBundle({ ...base, colors: { 'window.title': '#000000' } }));
	assert.throws(() => validateThemeBundle({ ...base, colors: { 'editor.background': 'red' } }));
	assert.throws(() => validateThemeBundle({ ...base, tokenColors: [{ scope: 'source', settings: { foreground: '#FFFFFF' } }] }));
	assert.throws(() => validateThemeBundle({ ...base, tokenColors: [{ scope: 'string', settings: { foreground: '#FFFFFF' } }, { scope: 'string', settings: { foreground: '#000000' } }] }));
});

test('exports only allowlisted current customizations', () => {
	const bundle = createThemeBundle('Soft Dusk', { 'editor.background': '#112233', 'window.title': '#FFFFFF' }, { textMateRules: [
		{ scope: 'comment', settings: { foreground: '#778899' } },
		{ scope: 'source', settings: { foreground: '#FFFFFF' } },
		{ scope: 'comment', settings: { foreground: '#000000' } }
	] });
	assert.deepEqual(bundle.colors, { 'editor.background': '#112233' });
	assert.deepEqual(bundle.tokenColors, [{ scope: 'comment', settings: { foreground: '#778899' } }]);
	assert.throws(() => createThemeBundle('Empty', {}, {}));
});
