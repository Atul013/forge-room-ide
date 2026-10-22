'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { contrastRatio } = require('../themeContrast');

const themeRoot = path.join(__dirname, '../themes');
const textPairs = [
	['foreground', 'sideBar.background'],
	['descriptionForeground', 'sideBar.background'],
	['editor.foreground', 'editor.background'],
	['editorLineNumber.foreground', 'editor.background'],
	['editorLineNumber.activeForeground', 'editor.background'],
	['sideBar.foreground', 'sideBar.background'],
	['sideBarTitle.foreground', 'sideBar.background'],
	['sideBarSectionHeader.foreground', 'sideBarSectionHeader.background'],
	['activityBar.foreground', 'activityBar.background'],
	['activityBarBadge.foreground', 'activityBarBadge.background'],
	['statusBar.foreground', 'statusBar.background'],
	['titleBar.activeForeground', 'titleBar.activeBackground'],
	['titleBar.inactiveForeground', 'titleBar.inactiveBackground'],
	['tab.activeForeground', 'tab.activeBackground'],
	['tab.inactiveForeground', 'tab.inactiveBackground'],
	['button.foreground', 'button.background'],
	['input.foreground', 'input.background'],
	['list.activeSelectionForeground', 'list.activeSelectionBackground'],
	['badge.foreground', 'badge.background'],
	['terminal.foreground', 'terminal.background']
];

test('every curated theme meets 4.5:1 for core workbench and syntax text', () => {
	const files = fs.readdirSync(themeRoot).filter(file => file.endsWith('-color-theme.json'));
	assert.equal(files.length, 6);
	for (const file of files) {
		const theme = JSON.parse(fs.readFileSync(path.join(themeRoot, file), 'utf8'));
		for (const [foreground, background] of textPairs) {
			const ratio = contrastRatio(theme.colors[foreground], theme.colors[background]);
			assert.ok(ratio >= 4.5, `${theme.name}: ${foreground} against ${background} is ${ratio.toFixed(2)}:1`);
		}
		for (const token of theme.tokenColors) {
			const ratio = contrastRatio(token.settings.foreground, theme.colors['editor.background']);
			assert.ok(ratio >= 4.5, `${theme.name}: ${token.scope.join(', ')} token is ${ratio.toFixed(2)}:1`);
		}
	}
});

test('focus and inactive activity icons keep at least 3:1 non-text contrast', () => {
	for (const file of fs.readdirSync(themeRoot).filter(file => file.endsWith('-color-theme.json'))) {
		const theme = JSON.parse(fs.readFileSync(path.join(themeRoot, file), 'utf8'));
		assert.ok(contrastRatio(theme.colors.focusBorder, theme.colors['editor.background']) >= 3, `${theme.name}: focus border against editor`);
		assert.ok(contrastRatio(theme.colors.focusBorder, theme.colors['sideBar.background']) >= 3, `${theme.name}: focus border against sidebar`);
		assert.ok(contrastRatio(theme.colors['activityBar.inactiveForeground'], theme.colors['activityBar.background']) >= 3, `${theme.name}: inactive activity icon`);
	}
});

test('contrast calculator rejects invalid colors rather than masking a bad theme value', () => {
	assert.throws(() => contrastRatio('red', '#FFFFFF'), /six-digit hex/);
	assert.throws(() => contrastRatio('#000000', '#FFF'), /six-digit hex/);
});
