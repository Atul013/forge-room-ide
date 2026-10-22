/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Forge Room. All rights reserved.
 *  Licensed under the MIT License. See LICENSE in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

'use strict';

const vscode = require('vscode');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { createThemeBundle, validateThemeBundle } = require('./themeBundles');
const { contrastRatio } = require('./themeContrast');
const { confirmThemePreview } = require('./themePreview');
const { createProfileSnapshot, mergeProfileSettings } = require('./profileSettings');

const setting = 'workbench.colorTheme';
const colorSetting = 'workbench.colorCustomizations';
const tokenSetting = 'editor.tokenColorCustomizations';
const colorIds = ['editor.background', 'editor.foreground', 'editor.selectionBackground', 'editor.lineHighlightBackground', 'activityBar.background', 'activityBar.foreground', 'sideBar.background', 'sideBar.foreground', 'statusBar.background', 'statusBar.foreground', 'focusBorder', 'editorCursor.foreground'];
const tokenScopes = { comments: 'comment', strings: 'string', keywords: 'keyword', numbers: 'constant.numeric', functions: 'entity.name.function', types: 'entity.name.type', variables: 'variable' };
const tokenDefaults = { comments: '#89848E', strings: '#B7C58A', keywords: '#D78BB2', numbers: '#E6B16F', functions: '#84BBC2', types: '#E0A180', variables: '#E4D9D8' };
const appearanceSettings = [setting, colorSetting, tokenSetting, 'workbench.iconTheme', 'workbench.productIconTheme', 'editor.fontFamily', 'editor.fontSize'];
const projectEditorSettings = ['editor.tabSize', 'editor.insertSpaces', 'editor.wordWrap', 'editor.minimap.enabled', 'editor.renderWhitespace', 'editor.lineNumbers', 'editor.rulers'];
const profileSettings = [...appearanceSettings, ...projectEditorSettings];
const restoreContext = 'forgeRoomStudio.hasPreviousTheme';

function validateColorMap(value) {
	if (!value || typeof value !== 'object' || Array.isArray(value) || Object.keys(value).length > colorIds.length) { throw new Error('Color customizations must be a small object of supported theme colors.'); }
	const colors = {};
	for (const [key, color] of Object.entries(value)) {
		if (!colorIds.includes(key)) { continue; }
		if (typeof color !== 'string' || !/^#[\da-f]{6}$/i.test(color)) { throw new Error(`Invalid hex color for ${key}. Use #RRGGBB.`); }
		colors[key] = color.toUpperCase();
	}
	if (!Object.keys(colors).length) { throw new Error('No supported color customizations were found.'); }
	return colors;
}

function validateTokenMap(value) {
	if (!value || typeof value !== 'object' || Array.isArray(value) || Object.keys(value).some(key => key !== 'textMateRules') || !Array.isArray(value.textMateRules) || value.textMateRules.length > Object.keys(tokenScopes).length) { throw new Error('Syntax colors must use the supported textMateRules format.'); }
	const allowedScopes = new Set(Object.values(tokenScopes));
	const rules = [];
	for (const rule of value.textMateRules) {
		if (!rule || typeof rule !== 'object' || Array.isArray(rule) || Object.keys(rule).some(key => !['scope', 'settings'].includes(key)) || typeof rule.scope !== 'string' || !allowedScopes.has(rule.scope) || !rule.settings || typeof rule.settings !== 'object' || Array.isArray(rule.settings) || Object.keys(rule.settings).some(key => key !== 'foreground') || typeof rule.settings.foreground !== 'string' || !/^#[\da-f]{6}$/i.test(rule.settings.foreground)) { throw new Error('A syntax-color rule contains an unsupported scope or value.'); }
		if (rules.some(item => item.scope === rule.scope)) { throw new Error(`Duplicate syntax-color scope: ${rule.scope}.`); }
		rules.push({ scope: rule.scope, settings: { foreground: rule.settings.foreground.toUpperCase() } });
	}
	if (!rules.length) { throw new Error('No supported syntax color rules were found.'); }
	return { textMateRules: rules };
}

class ThemeStudioProvider {
	view;
	catalog;
	constructor(context) {
		this.context = context;
		this.catalog = JSON.parse(fs.readFileSync(path.join(context.extensionPath, 'themes', 'catalog.json'), 'utf8'));
		context.subscriptions.push(vscode.workspace.onDidChangeConfiguration(event => {
			if (appearanceSettings.some(key => event.affectsConfiguration(key))) { this.render(); }
		}));
	}

	async resolveWebviewView(view) {
		this.view = view;
		view.webview.options = { enableScripts: true, localResourceRoots: [vscode.Uri.joinPath(this.context.extensionUri, 'media'), vscode.Uri.joinPath(this.context.extensionUri, 'themes')] };
		view.webview.html = this.html(view.webview);
		view.webview.onDidReceiveMessage(async message => {
			if (message?.command === 'ready') { await this.render(); }
			if (message?.command === 'apply' && typeof message.id === 'string') { await this.apply(message.id); }
			if (message?.command === 'restore') { await this.restore(); }
		});
		await this.render();
	}

	scope() {
		const workspace = vscode.workspace.workspaceFolders?.[0];
		return workspace
			? { id: `workspace:${workspace.uri.toString()}`, target: vscode.ConfigurationTarget.Workspace }
			: { id: 'user', target: vscode.ConfigurationTarget.Global };
	}

	storageKey(scope) { return `forgeRoomStudio.previousTheme.${crypto.createHash('sha256').update(scope).digest('hex').slice(0, 20)}`; }

	async updateRestoreContext() {
		const workspace = vscode.workspace.workspaceFolders?.[0];
		const keys = [this.storageKey('user'), ...(workspace ? [this.storageKey(`workspace:${workspace.uri.toString()}`)] : [])];
		await vscode.commands.executeCommand('setContext', restoreContext, keys.some(key => !!this.context.globalState.get(key)));
	}

	savedValues(value) {
		if (value && value.settings && typeof value.settings === 'object') { return value.settings; }
		if (value && Object.prototype.hasOwnProperty.call(value, 'value')) { return { [setting]: { target: value.target, value: value.value } }; }
		return {};
	}

	async capturePrevious(scope, keys) {
		const storageKey = this.storageKey(scope.id);
		const current = {};
		const configuration = vscode.workspace.getConfiguration();
		for (const key of keys) {
			const inspected = configuration.inspect(key);
			current[key] = scope.target === vscode.ConfigurationTarget.Workspace ? inspected?.workspaceValue : inspected?.globalValue;
		}
		await this.context.globalState.update(storageKey, createProfileSnapshot(keys, current, scope.target));
	}

	async apply(id) {
		const theme = this.catalog.themes.find(item => item.id === id);
		if (!theme) { return; }
		const scope = this.scope();
		await this.capturePrevious(scope, [setting]);
		try {
			await this.updateRestoreContext();
			const kept = await confirmThemePreview(
				() => vscode.workspace.getConfiguration().update(setting, theme.id, scope.target),
				async () => (await vscode.window.showInformationMessage(`Previewing ${theme.name}. Keep this theme or restore the previous appearance?`, { modal: true }, 'Keep theme', 'Restore previous')) === 'Keep theme',
				() => this.restore(scope)
			);
			if (kept) {
				await this.updateRestoreContext();
				void vscode.window.setStatusBarMessage(`Forge Room theme kept: ${theme.name}`, 3000);
			}
		} catch (error) {
			void vscode.window.showErrorMessage(`Could not apply theme: ${String(error)}`);
		}
	}

	async restore(requestedScope) {
		let scope = requestedScope;
		if (!scope) {
			const options = [];
			const userScope = { id: 'user', target: vscode.ConfigurationTarget.Global };
			if (this.context.globalState.get(this.storageKey(userScope.id))) { options.push({ label: 'User settings', scope: userScope }); }
			for (const workspace of vscode.workspace.workspaceFolders || []) {
				const workspaceScope = { id: `workspace:${workspace.uri.toString()}`, target: vscode.ConfigurationTarget.Workspace };
				if (this.context.globalState.get(this.storageKey(workspaceScope.id))) { options.push({ label: `Workspace: ${workspace.name}`, description: workspace.uri.fsPath, scope: workspaceScope }); }
			}
			if (options.length > 1) {
				const selected = await vscode.window.showQuickPick(options, { placeHolder: 'Choose which previous appearance to restore' });
				if (!selected) { return; }
				scope = selected.scope;
			} else { scope = options[0]?.scope; }
		}
		if (!scope) { return; }
		const key = this.storageKey(scope.id);
		const saved = this.context.globalState.get(key);
		if (!saved) { return; }
		try {
			const settings = this.savedValues(saved);
			for (const [name, entry] of Object.entries(settings)) {
				if (!profileSettings.includes(name) || !entry || ![vscode.ConfigurationTarget.Global, vscode.ConfigurationTarget.Workspace].includes(entry.target)) { continue; }
				await vscode.workspace.getConfiguration().update(name, entry.value === null ? undefined : entry.value, entry.target);
			}
			await this.context.globalState.update(key, undefined);
			const paletteKey = `forgeRoomStudio.paletteName.${crypto.createHash('sha256').update(scope.id).digest('hex').slice(0, 20)}`;
			await this.context.workspaceState.update(paletteKey, undefined);
			await this.updateRestoreContext();
			void vscode.window.setStatusBarMessage('Previous appearance restored', 3000);
		} catch (error) {
			void vscode.window.showErrorMessage(`Could not restore the previous theme: ${String(error)}`);
		}
	}

	async customizePalette() {
		const paletteName = await vscode.window.showInputBox({ prompt: 'Name this custom palette', value: 'My Forge Room palette', validateInput: value => value.trim().length > 0 && value.length <= 60 ? undefined : 'Enter a name of up to 60 characters' });
		if (!paletteName) { return; }
		const existing = vscode.workspace.getConfiguration().get(colorSetting) || {};
		const askColor = (label, key, fallback) => vscode.window.showInputBox({ prompt: `${label} (${key})`, value: typeof existing[key] === 'string' && /^#[\da-f]{6}$/i.test(existing[key]) ? existing[key] : fallback, validateInput: value => /^#[\da-f]{6}$/i.test(value.trim()) ? undefined : 'Use #RRGGBB' });
		const background = await askColor('Editor background', 'editor.background', '#1E1E1E');
		if (!background) { return; }
		const foreground = await askColor('Editor text', 'editor.foreground', '#D4D4D4');
		if (!foreground) { return; }
		const accent = await askColor('Accent', 'focusBorder', '#7C3AED');
		if (!accent) { return; }
		const selection = await askColor('Selected text background', 'editor.selectionBackground', '#264F78');
		if (!selection) { return; }
		const accentText = await askColor('Accent text', 'statusBar.foreground', '#FFFFFF');
		if (!accentText) { return; }
		const contrastIssues = [];
		if (contrastRatio(foreground.trim(), background.trim()) < 4.5) { contrastIssues.push('editor text against editor background'); }
		if (contrastRatio(foreground.trim(), selection.trim()) < 4.5) { contrastIssues.push('editor text against selected-text background'); }
		if (contrastRatio(accentText.trim(), accent.trim()) < 4.5) { contrastIssues.push('accent text against accent surfaces'); }
		if (contrastIssues.length) {
			const proceed = await vscode.window.showWarningMessage(`This palette has low text contrast for ${contrastIssues.join(' and ')}. Preview it anyway and review readability?`, { modal: true }, 'Preview anyway');
			if (proceed !== 'Preview anyway') { return; }
		}
		const targets = [];
		if (vscode.workspace.workspaceFolders?.length) { targets.push({ label: 'This workspace', description: 'Only the open project', target: vscode.ConfigurationTarget.Workspace }); }
		targets.push({ label: 'User settings', description: 'All projects for this IDE profile', target: vscode.ConfigurationTarget.Global });
		const choice = await vscode.window.showQuickPick(targets, { placeHolder: 'Choose where to preview and save this palette' });
		if (!choice) { return; }
		const scope = choice.target === vscode.ConfigurationTarget.Workspace ? { id: `workspace:${vscode.workspace.workspaceFolders[0].uri.toString()}`, target: choice.target } : { id: 'user', target: choice.target };
		const colors = {
			'editor.background': background.trim().toUpperCase(),
			'editor.foreground': foreground.trim().toUpperCase(),
			'editor.selectionBackground': selection.trim().toUpperCase(),
			'activityBar.background': accent.trim().toUpperCase(),
			'activityBar.foreground': accentText.trim().toUpperCase(),
			'statusBar.background': accent.trim().toUpperCase(),
			'statusBar.foreground': accentText.trim().toUpperCase(),
			'focusBorder': accent.trim().toUpperCase()
		};
		const confirmation = await vscode.window.showWarningMessage(`Preview “${paletteName.trim()}” at ${choice.label}? This changes only allowlisted color customizations.`, { modal: true }, 'Preview palette');
		if (confirmation !== 'Preview palette') { return; }
		const configuration = vscode.workspace.getConfiguration();
		const previousSnapshot = this.context.globalState.get(this.storageKey(scope.id));
		const inspected = configuration.inspect(colorSetting);
		const previousValue = scope.target === vscode.ConfigurationTarget.Workspace ? inspected?.workspaceValue : inspected?.globalValue;
		await this.capturePrevious(scope, [colorSetting]);
		try {
			const targetColors = previousValue && typeof previousValue === 'object' && !Array.isArray(previousValue) ? previousValue : {};
			await configuration.update(colorSetting, { ...targetColors, ...colors }, scope.target);
			const keep = await vscode.window.showInformationMessage(`Keep the previewed palette “${paletteName.trim()}”?`, { modal: true }, 'Keep palette', 'Restore previous colors');
			if (keep !== 'Keep palette') {
				await configuration.update(colorSetting, previousValue === undefined ? undefined : previousValue, scope.target);
				if (!previousSnapshot) { await this.context.globalState.update(this.storageKey(scope.id), undefined); }
				await this.updateRestoreContext();
				return;
			}
			await this.context.workspaceState.update(`forgeRoomStudio.paletteName.${crypto.createHash('sha256').update(scope.id).digest('hex').slice(0, 20)}`, paletteName.trim());
			await this.updateRestoreContext();
			void vscode.window.setStatusBarMessage(`Custom palette applied: ${paletteName.trim()}`, 4000);
		} catch (error) {
			await configuration.update(colorSetting, previousValue === undefined ? undefined : previousValue, scope.target);
			if (!previousSnapshot) { await this.context.globalState.update(this.storageKey(scope.id), undefined); }
			await this.updateRestoreContext();
			void vscode.window.showErrorMessage(`Could not apply custom palette: ${String(error)}`);
		}
	}

	async customizeSyntaxPalette() {
		const paletteName = await vscode.window.showInputBox({ prompt: 'Name this syntax palette', value: 'My Forge Room syntax palette', validateInput: value => value.trim().length > 0 && value.length <= 60 ? undefined : 'Enter a name of up to 60 characters' });
		if (!paletteName) { return; }
		const configuration = vscode.workspace.getConfiguration();
		const current = configuration.get(tokenSetting) || {};
		const currentRules = Array.isArray(current.textMateRules) ? current.textMateRules : [];
		const currentThemeId = configuration.get(setting);
		const themeMeta = this.catalog.themes.find(theme => theme.id === currentThemeId);
		let themeTokens = [];
		let themeBackground;
		if (themeMeta) {
			try {
				const themeFile = path.join(this.context.extensionPath, 'themes', `${themeMeta.key}-color-theme.json`);
				const theme = JSON.parse(await fs.promises.readFile(themeFile, 'utf8'));
				themeTokens = Array.isArray(theme.tokenColors) ? theme.tokenColors : [];
				themeBackground = theme.colors?.['editor.background'];
			} catch { /* Continue with readable defaults if the selected theme cannot be read. */ }
		}
		const currentColors = configuration.get(colorSetting) || {};
		const background = /^#[\da-f]{6}$/i.test(currentColors['editor.background'] || '') ? currentColors['editor.background'] : themeBackground || '#1E1E1E';
		const tokenColor = (key, scope) => {
			const override = currentRules.find(rule => rule.scope === scope)?.settings?.foreground;
			if (typeof override === 'string' && /^#[\da-f]{6}$/i.test(override)) { return override; }
			const themeRule = themeTokens.find(rule => (Array.isArray(rule.scope) ? rule.scope : [rule.scope]).includes(scope));
			return typeof themeRule?.settings?.foreground === 'string' && /^#[\da-f]{6}$/i.test(themeRule.settings.foreground) ? themeRule.settings.foreground : tokenDefaults[key];
		};
		const colors = {};
		const contrastIssues = [];
		for (const [key, scopeName] of Object.entries(tokenScopes)) {
			const label = key[0].toUpperCase() + key.slice(1);
			const value = await vscode.window.showInputBox({ prompt: `${label} syntax color (${scopeName})`, value: tokenColor(key, scopeName), validateInput: candidate => /^#[\da-f]{6}$/i.test(candidate.trim()) ? undefined : 'Use #RRGGBB' });
			if (!value) { return; }
			colors[scopeName] = value.trim().toUpperCase();
			if (contrastRatio(value.trim(), background) < 4.5) { contrastIssues.push(label); }
		}
		if (contrastIssues.length) {
			const proceed = await vscode.window.showWarningMessage(`These syntax colors may be difficult to read against the current editor background: ${contrastIssues.join(', ')}. Preview them anyway?`, { modal: true }, 'Preview anyway');
			if (proceed !== 'Preview anyway') { return; }
		}
		const targets = [];
		if (vscode.workspace.workspaceFolders?.length) { targets.push({ label: 'This workspace', description: 'Only the open project', target: vscode.ConfigurationTarget.Workspace }); }
		targets.push({ label: 'User settings', description: 'All projects for this IDE profile', target: vscode.ConfigurationTarget.Global });
		const choice = await vscode.window.showQuickPick(targets, { placeHolder: 'Choose where to preview and save these syntax colors' });
		if (!choice) { return; }
		const scope = choice.target === vscode.ConfigurationTarget.Workspace ? { id: `workspace:${vscode.workspace.workspaceFolders[0].uri.toString()}`, target: choice.target } : { id: 'user', target: choice.target };
		const confirmation = await vscode.window.showWarningMessage(`Preview the syntax palette “${paletteName.trim()}” at ${choice.label}?`, { modal: true }, 'Preview palette');
		if (confirmation !== 'Preview palette') { return; }
		const previousSnapshot = this.context.globalState.get(this.storageKey(scope.id));
		const inspected = configuration.inspect(tokenSetting);
		const previousValue = scope.target === vscode.ConfigurationTarget.Workspace ? inspected?.workspaceValue : inspected?.globalValue;
		await this.capturePrevious(scope, [tokenSetting]);
		try {
			const targetValue = previousValue && typeof previousValue === 'object' && !Array.isArray(previousValue) ? previousValue : {};
			const scopesToReplace = new Set(Object.values(tokenScopes));
			const retained = (Array.isArray(targetValue.textMateRules) ? targetValue.textMateRules : []).flatMap(rule => {
				const scopes = Array.isArray(rule?.scope) ? rule.scope : [rule?.scope];
				const remaining = scopes.filter(item => !scopesToReplace.has(item));
				return remaining.length ? [{ ...rule, scope: Array.isArray(rule.scope) ? remaining : remaining[0] }] : [];
			});
			const rules = Object.entries(colors).map(([scopeName, foreground]) => ({ scope: scopeName, settings: { foreground } }));
			await configuration.update(tokenSetting, { ...targetValue, textMateRules: [...retained, ...rules] }, scope.target);
			const keep = await vscode.window.showInformationMessage(`Keep the previewed syntax palette “${paletteName.trim()}”?`, { modal: true }, 'Keep palette', 'Restore previous syntax colors');
			if (keep !== 'Keep palette') {
				await configuration.update(tokenSetting, previousValue === undefined ? undefined : previousValue, scope.target);
				if (!previousSnapshot) { await this.context.globalState.update(this.storageKey(scope.id), undefined); }
				await this.updateRestoreContext();
				return;
			}
			await this.context.workspaceState.update(`forgeRoomStudio.paletteName.${crypto.createHash('sha256').update(scope.id).digest('hex').slice(0, 20)}`, paletteName.trim());
			await this.updateRestoreContext();
			void vscode.window.setStatusBarMessage(`Custom syntax palette applied: ${paletteName.trim()}`, 4000);
		} catch (error) {
			await configuration.update(tokenSetting, previousValue === undefined ? undefined : previousValue, scope.target);
			if (!previousSnapshot) { await this.context.globalState.update(this.storageKey(scope.id), undefined); }
			await this.updateRestoreContext();
			void vscode.window.showErrorMessage(`Could not apply syntax palette: ${String(error)}`);
		}
	}

	async exportProfile() {
		const scope = this.scope();
		const paletteKey = `forgeRoomStudio.paletteName.${crypto.createHash('sha256').update(scope.id).digest('hex').slice(0, 20)}`;
		const name = await vscode.window.showInputBox({ prompt: 'Name this appearance profile', value: this.context.workspaceState.get(paletteKey) || vscode.workspace.workspaceFolders?.[0]?.name || 'Forge Room Profile', validateInput: value => value.trim().length > 0 && value.length <= 80 ? undefined : 'Enter a profile name of up to 80 characters' });
		if (!name) { return; }
		const configuration = vscode.workspace.getConfiguration();
		const settings = {};
		for (const key of profileSettings) {
			const value = configuration.get(key);
			if (key === tokenSetting && value && typeof value === 'object' && Array.isArray(value.textMateRules)) {
				const allowedScopes = new Set(Object.values(tokenScopes));
				const rules = value.textMateRules.filter(rule => typeof rule?.scope === 'string' && allowedScopes.has(rule.scope) && typeof rule.settings?.foreground === 'string' && /^#[\da-f]{6}$/i.test(rule.settings.foreground)).map(rule => ({ scope: rule.scope, settings: { foreground: rule.settings.foreground.toUpperCase() } }));
				if (rules.length) { settings[key] = { textMateRules: rules }; }
			} else if (key === colorSetting && value && typeof value === 'object' && !Array.isArray(value)) {
				const colors = {};
				for (const colorId of colorIds) { if (typeof value[colorId] === 'string' && /^#[\da-f]{6}$/i.test(value[colorId])) { colors[colorId] = value[colorId].toUpperCase(); } }
				if (Object.keys(colors).length) { settings[key] = colors; }
			} else if (key !== colorSetting && value !== undefined) { settings[key] = value; }
		}
		const safeName = name.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'forge-room-profile';
		const workspace = vscode.workspace.workspaceFolders?.[0];
		const defaultUri = workspace ? vscode.Uri.file(path.join(workspace.uri.fsPath, `${safeName}.forge-room-profile.json`)) : undefined;
		const uri = await vscode.window.showSaveDialog({ defaultUri, saveLabel: 'Export Appearance Profile', filters: { 'Forge Room profile': ['json'] } });
		if (!uri) { return; }
		const profile = { format: 'forge-room-project-profile', version: 2, name: name.trim(), exportedAt: new Date().toISOString(), settings };
		await fs.promises.writeFile(uri.fsPath, `${JSON.stringify(profile, null, 2)}\n`, { encoding: 'utf8', flag: 'wx' }).catch(async error => {
			if (error.code !== 'EEXIST') { throw error; }
			await fs.promises.writeFile(uri.fsPath, `${JSON.stringify(profile, null, 2)}\n`, 'utf8');
		});
		void vscode.window.setStatusBarMessage(`Appearance profile exported: ${path.basename(uri.fsPath)}`, 4000);
	}

	async exportThemeBundle() {
		const name = await vscode.window.showInputBox({ prompt: 'Name this reusable theme bundle', validateInput: value => value.trim().length > 0 && value.length <= 80 ? undefined : 'Enter a theme name of up to 80 characters' });
		if (!name) { return; }
		let bundle;
		try {
			const configuration = vscode.workspace.getConfiguration();
			bundle = createThemeBundle(name.trim(), configuration.get(colorSetting), configuration.get(tokenSetting));
		} catch (error) { void vscode.window.showInformationMessage('No supported custom theme colors are active to export: ' + String(error)); return; }
		const safeName = name.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'forge-room-theme';
		const workspace = vscode.workspace.workspaceFolders?.[0];
		const defaultUri = workspace ? vscode.Uri.file(path.join(workspace.uri.fsPath, safeName + '.forge-room-theme.json')) : undefined;
		const uri = await vscode.window.showSaveDialog({ defaultUri, saveLabel: 'Export Theme Bundle', filters: { 'Forge Room theme bundle': ['json'] } });
		if (!uri) { return; }
		await fs.promises.writeFile(uri.fsPath, JSON.stringify(bundle, null, 2) + '\n', { encoding: 'utf8', flag: 'wx' }).catch(async error => {
			if (error.code !== 'EEXIST') { throw error; }
			await fs.promises.writeFile(uri.fsPath, JSON.stringify(bundle, null, 2) + '\n', 'utf8');
		});
		void vscode.window.setStatusBarMessage('Theme bundle exported: ' + path.basename(uri.fsPath), 4000);
	}

	async importThemeBundle() {
		const [uri] = await vscode.window.showOpenDialog({ canSelectMany: false, openLabel: 'Review Theme Bundle', filters: { 'Forge Room theme bundle': ['json'] } }) || [];
		if (!uri) { return; }
		try {
			const stat = await fs.promises.stat(uri.fsPath);
			if (!stat.isFile() || stat.size > 128 * 1024) { throw new Error('Theme bundles must be smaller than 128 KB.'); }
			const bundle = JSON.parse(await fs.promises.readFile(uri.fsPath, 'utf8'));
			const validated = validateThemeBundle(bundle);
			const names = Object.keys(validated.settings).join(', ');
			const scope = this.scope();
			const configuration = vscode.workspace.getConfiguration();
			const importedColors = validated.settings[colorSetting] || {};
			const currentColors = configuration.get(colorSetting) || {};
			const background = importedColors['editor.background'] || (typeof currentColors['editor.background'] === 'string' && /^#[\da-f]{6}$/i.test(currentColors['editor.background']) ? currentColors['editor.background'] : undefined);
			const foreground = importedColors['editor.foreground'] || (typeof currentColors['editor.foreground'] === 'string' && /^#[\da-f]{6}$/i.test(currentColors['editor.foreground']) ? currentColors['editor.foreground'] : undefined);
			const contrastIssues = [];
			if (background && foreground && contrastRatio(background, foreground) < 4.5) { contrastIssues.push('editor foreground'); }
			if (background) {
				for (const rule of validated.settings[tokenSetting]?.textMateRules || []) {
					if (contrastRatio(background, rule.settings.foreground) < 4.5) { contrastIssues.push(rule.scope); }
				}
			}
			const contrastNote = contrastIssues.length ? ' Potential low contrast against the current/imported editor background: ' + contrastIssues.join(', ') + '.' : '';
			const approval = await vscode.window.showWarningMessage('Preview theme bundle ' + validated.name + ' in ' + (scope.id === 'user' ? 'user settings' : 'this workspace') + '? It changes only these supported customization groups: ' + names + '. The active base theme stays selected; bundle code and unsupported theme properties are not applied.' + contrastNote, { modal: true }, 'Preview Theme Bundle');
			if (approval !== 'Preview Theme Bundle') { return; }
			const previousSnapshot = this.context.globalState.get(this.storageKey(scope.id));
			await this.capturePrevious(scope, Object.keys(validated.settings));
			const previousValues = Object.fromEntries(Object.keys(validated.settings).map(key => {
				const inspected = configuration.inspect(key);
				return [key, scope.target === vscode.ConfigurationTarget.Workspace ? inspected?.workspaceValue : inspected?.globalValue];
			}));
			try {
				for (const [key, value] of Object.entries(validated.settings)) {
					const inspected = configuration.inspect(key);
					const current = scope.target === vscode.ConfigurationTarget.Workspace ? inspected?.workspaceValue : inspected?.globalValue;
					if (key === colorSetting) {
						await configuration.update(key, { ...(current && typeof current === 'object' && !Array.isArray(current) ? current : {}), ...value }, scope.target);
					} else if (key === tokenSetting) {
						const previousRules = current?.textMateRules && Array.isArray(current.textMateRules) ? current.textMateRules : [];
						const replacing = new Set(value.textMateRules.map(rule => rule.scope));
						const retained = previousRules.filter(rule => !replacing.has(rule?.scope));
						await configuration.update(key, { ...(current && typeof current === 'object' && !Array.isArray(current) ? current : {}), textMateRules: [...retained, ...value.textMateRules] }, scope.target);
					}
				}
				const keep = await vscode.window.showWarningMessage('Keep the previewed theme bundle ' + validated.name + '?', { modal: true }, 'Keep Theme Bundle', 'Restore Previous');
				if (keep !== 'Keep Theme Bundle') {
					for (const [key, value] of Object.entries(previousValues)) { await configuration.update(key, value, scope.target); }
					if (!previousSnapshot) { await this.context.globalState.update(this.storageKey(scope.id), undefined); }
					await this.updateRestoreContext();
					return;
				}
				const paletteKey = 'forgeRoomStudio.paletteName.' + crypto.createHash('sha256').update(scope.id).digest('hex').slice(0, 20);
				await this.context.workspaceState.update(paletteKey, validated.name);
				await this.updateRestoreContext();
				void vscode.window.setStatusBarMessage('Theme bundle applied: ' + validated.name, 4000);
			} catch (error) {
				for (const [key, value] of Object.entries(previousValues)) { await configuration.update(key, value, scope.target); }
				if (!previousSnapshot) { await this.context.globalState.update(this.storageKey(scope.id), undefined); }
				await this.updateRestoreContext();
				throw error;
			}
		} catch (error) { void vscode.window.showErrorMessage('Could not import theme bundle: ' + String(error)); }
	}

	validateProfile(profile) {
		const legacy = profile?.format === 'forge-room-appearance' && profile.version === 1;
		if (!profile || (!legacy && (profile.format !== 'forge-room-project-profile' || profile.version !== 2)) || typeof profile.name !== 'string' || profile.name.trim().length === 0 || profile.name.length > 80 || !profile.settings || typeof profile.settings !== 'object' || Array.isArray(profile.settings) || Object.keys(profile.settings).length > profileSettings.length) { throw new Error('This file is not a supported Forge Room project profile.'); }
		const valid = {};
		for (const [key, value] of Object.entries(profile.settings)) {
			if (!profileSettings.includes(key)) { continue; }
			if (key === colorSetting) { valid[key] = validateColorMap(value); continue; }
			if (key === tokenSetting) { valid[key] = validateTokenMap(value); continue; }
			if (value === null && ['workbench.iconTheme', 'workbench.productIconTheme'].includes(key)) { valid[key] = value; continue; }
			if (key === 'editor.fontSize' && typeof value === 'number' && Number.isFinite(value) && value >= 6 && value <= 96) { valid[key] = value; continue; }
			if (key === 'editor.tabSize' && Number.isInteger(value) && value >= 1 && value <= 16) { valid[key] = value; continue; }
			if (['editor.insertSpaces', 'editor.minimap.enabled'].includes(key) && typeof value === 'boolean') { valid[key] = value; continue; }
			if (key === 'editor.wordWrap' && ['off', 'on', 'wordWrapColumn', 'bounded'].includes(value)) { valid[key] = value; continue; }
			if (key === 'editor.renderWhitespace' && ['none', 'boundary', 'selection', 'trailing', 'all'].includes(value)) { valid[key] = value; continue; }
			if (key === 'editor.lineNumbers' && ['off', 'on', 'relative', 'interval'].includes(value)) { valid[key] = value; continue; }
			if (key === 'editor.rulers' && Array.isArray(value) && value.length <= 8 && value.every(item => Number.isInteger(item) && item >= 1 && item <= 500)) { valid[key] = [...value]; continue; }
			if (key !== 'editor.fontSize' && typeof value === 'string' && value.length <= 128) { valid[key] = value; continue; }
			throw new Error(`Invalid value for ${key}.`);
		}
		if (!Object.keys(valid).length) { throw new Error('The profile does not contain supported appearance settings.'); }
		return valid;
	}

	async importProfile() {
		const [uri] = await vscode.window.showOpenDialog({ canSelectMany: false, openLabel: 'Review Appearance Profile', filters: { 'Forge Room profile': ['json'] } }) || [];
		if (!uri) { return; }
		try {
			const stat = await fs.promises.stat(uri.fsPath);
			if (!stat.isFile() || stat.size > 128 * 1024) { throw new Error('Profile files must be smaller than 128 KB.'); }
			const profile = JSON.parse(await fs.promises.readFile(uri.fsPath, 'utf8'));
			const settings = this.validateProfile(profile);
			const names = Object.keys(settings).join(', ');
			const approval = await vscode.window.showWarningMessage(`Temporarily preview profile '${profile.name}' in ${this.scope().id === 'user' ? 'user settings' : 'this workspace'}? You can keep it or restore the previous values. Only these allowlisted appearance and editor preferences will change: ${names}.`, { modal: true }, 'Preview Profile');
			if (approval !== 'Preview Profile') { return; }
			const scope = this.scope();
			const configuration = vscode.workspace.getConfiguration();
			const current = {};
			for (const key of Object.keys(settings)) {
				const inspected = configuration.inspect(key);
				current[key] = scope.target === vscode.ConfigurationTarget.Workspace ? inspected?.workspaceValue : inspected?.globalValue;
			}
			const previewSettings = mergeProfileSettings(settings, current);
			await this.capturePrevious(scope, Object.keys(previewSettings));
			try {
				await this.updateRestoreContext();
				const kept = await confirmThemePreview(
					async () => { for (const [key, value] of Object.entries(previewSettings)) { await configuration.update(key, value, scope.target); } },
					async () => (await vscode.window.showWarningMessage(`Previewing profile '${profile.name}'. Keep the profile or restore the previous settings?`, { modal: true }, 'Keep Profile', 'Restore Previous')) === 'Keep Profile',
					() => this.restore(scope)
				);
				if (!kept) { return; }
				if (settings[colorSetting] || settings[tokenSetting]) {
					const paletteKey = `forgeRoomStudio.paletteName.${crypto.createHash('sha256').update(scope.id).digest('hex').slice(0, 20)}`;
					await this.context.workspaceState.update(paletteKey, profile.name);
				}
				await this.updateRestoreContext();
				void vscode.window.setStatusBarMessage(`Appearance profile kept: ${profile.name}`, 4000);
			} catch (error) {
				await this.restore(scope);
				throw error;
			}
		} catch (error) {
			void vscode.window.showErrorMessage(`Could not import appearance profile: ${String(error)}`);
		}
	}

	async render() {
		if (!this.view) { return; }
		const current = vscode.workspace.getConfiguration().get(setting);
		const scope = this.scope();
		const previous = [scope.id, 'user'].some(id => !!this.context.globalState.get(this.storageKey(id)));
		await this.view.webview.postMessage({ type: 'studio-state', current, previous: !!previous, themes: this.catalog.themes });
	}

	html(webview) {
		const nonce = crypto.randomBytes(16).toString('base64');
		const style = webview.asWebviewUri(vscode.Uri.joinPath(this.context.extensionUri, 'media', 'studio.css'));
		const script = webview.asWebviewUri(vscode.Uri.joinPath(this.context.extensionUri, 'media', 'studio.js'));
		const initial = JSON.stringify({ themes: this.catalog.themes }).replace(/</g, '\\u003c');
		return `<!doctype html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';"><link rel="stylesheet" href="${style}"><title>Forge Room Studio</title></head><body><div id="studio"></div><script nonce="${nonce}">window.studioInitial=${initial};window.forgeStudio=acquireVsCodeApi();</script><script nonce="${nonce}" src="${script}"></script></body></html>`;
	}
}

function activate(context) {
	const provider = new ThemeStudioProvider(context);
	context.subscriptions.push(
		vscode.window.registerWebviewViewProvider('forgeRoomStudio.gallery', provider),
		vscode.commands.registerCommand('forgeRoomStudio.open', () => vscode.commands.executeCommand('forgeRoomStudio.gallery.focus')),
		vscode.commands.registerCommand('forgeRoomStudio.restore', () => provider.restore()),
		vscode.commands.registerCommand('forgeRoomStudio.customizePalette', () => provider.customizePalette()),
		vscode.commands.registerCommand('forgeRoomStudio.customizeSyntaxPalette', () => provider.customizeSyntaxPalette()),
		vscode.commands.registerCommand('forgeRoomStudio.exportProfile', () => provider.exportProfile()),
		vscode.commands.registerCommand('forgeRoomStudio.importProfile', () => provider.importProfile()),
	vscode.commands.registerCommand('forgeRoomStudio.exportThemeBundle', () => provider.exportThemeBundle()),
	vscode.commands.registerCommand('forgeRoomStudio.importThemeBundle', () => provider.importThemeBundle()),
		vscode.workspace.onDidChangeWorkspaceFolders(() => void provider.updateRestoreContext())
	);
	void provider.updateRestoreContext();
}

function deactivate() {}

module.exports = { activate, deactivate };
