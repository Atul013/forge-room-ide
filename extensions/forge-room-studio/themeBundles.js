'use strict';

const colorSetting = 'workbench.colorCustomizations';
const tokenSetting = 'editor.tokenColorCustomizations';
const colorIds = ['editor.background', 'editor.foreground', 'editor.selectionBackground', 'editor.lineHighlightBackground', 'activityBar.background', 'activityBar.foreground', 'sideBar.background', 'sideBar.foreground', 'statusBar.background', 'statusBar.foreground', 'focusBorder', 'editorCursor.foreground'];
const tokenScopes = new Set(['comment', 'string', 'keyword', 'constant.numeric', 'entity.name.function', 'entity.name.type', 'variable']);
const validColor = value => typeof value === 'string' && /^#[\da-f]{6}$/i.test(value);

function validateThemeBundle(bundle) {
	if (!bundle || typeof bundle !== 'object' || Array.isArray(bundle)
		|| Object.keys(bundle).some(key => !['format', 'version', 'name', 'colors', 'tokenColors'].includes(key))
		|| bundle.format !== 'forge-room-theme-bundle' || bundle.version !== 1
		|| typeof bundle.name !== 'string' || !bundle.name.trim() || bundle.name.length > 80) {
		throw new Error('This is not a supported Forge Room theme bundle.');
	}
	const colors = {};
	if (bundle.colors !== undefined) {
		if (!bundle.colors || typeof bundle.colors !== 'object' || Array.isArray(bundle.colors) || Object.keys(bundle.colors).length > colorIds.length) {
			throw new Error('Theme colors must be an object containing only supported color IDs.');
		}
		for (const [id, color] of Object.entries(bundle.colors)) {
			if (!colorIds.includes(id) || !validColor(color)) { throw new Error('Unsupported theme color or invalid hex value: ' + id + '.'); }
			colors[id] = color.toUpperCase();
		}
	}
	const tokenColors = [];
	if (bundle.tokenColors !== undefined) {
		if (!Array.isArray(bundle.tokenColors) || bundle.tokenColors.length > tokenScopes.size) { throw new Error('Theme tokenColors must contain at most seven supported scopes.'); }
		for (const rule of bundle.tokenColors) {
			if (!rule || typeof rule !== 'object' || Array.isArray(rule)
				|| Object.keys(rule).some(key => !['scope', 'settings'].includes(key))
				|| typeof rule.scope !== 'string' || !tokenScopes.has(rule.scope)
				|| !rule.settings || typeof rule.settings !== 'object' || Array.isArray(rule.settings)
				|| Object.keys(rule.settings).some(key => key !== 'foreground')
				|| !validColor(rule.settings.foreground)) {
				throw new Error('A theme token rule uses an unsupported scope or value.');
			}
			if (tokenColors.some(item => item.scope === rule.scope)) { throw new Error('Duplicate theme token scope: ' + rule.scope + '.'); }
			tokenColors.push({ scope: rule.scope, settings: { foreground: rule.settings.foreground.toUpperCase() } });
		}
	}
	if (!Object.keys(colors).length && !tokenColors.length) { throw new Error('The theme bundle has no supported colors.'); }
	return {
		name: bundle.name.trim(),
		settings: {
			...(Object.keys(colors).length ? { [colorSetting]: colors } : {}),
			...(tokenColors.length ? { [tokenSetting]: { textMateRules: tokenColors } } : {})
		}
	};
}

function createThemeBundle(name, colors, tokenSettings) {
	const safeColors = {};
	for (const id of colorIds) { if (validColor(colors?.[id])) { safeColors[id] = colors[id].toUpperCase(); } }
	const sourceRules = Array.isArray(tokenSettings?.textMateRules) ? tokenSettings.textMateRules : [];
	const tokenColors = [];
	for (const rule of sourceRules) {
		if (rule && tokenScopes.has(rule.scope) && validColor(rule.settings?.foreground) && !tokenColors.some(item => item.scope === rule.scope)) {
			tokenColors.push({ scope: rule.scope, settings: { foreground: rule.settings.foreground.toUpperCase() } });
		}
	}
	if (!name || typeof name !== 'string' || name.trim().length > 80 || (!Object.keys(safeColors).length && !tokenColors.length)) {
		throw new Error('Provide a name and at least one supported color to export a theme bundle.');
	}
	return { format: 'forge-room-theme-bundle', version: 1, name: name.trim(), colors: safeColors, tokenColors };
}

module.exports = { createThemeBundle, validateThemeBundle };
