'use strict';

const colorSetting = 'workbench.colorCustomizations';
const tokenSetting = 'editor.tokenColorCustomizations';

function mergeProfileSettings(imported, current) {
	const result = { ...imported };
	if (imported[colorSetting] && current[colorSetting] && typeof current[colorSetting] === 'object' && !Array.isArray(current[colorSetting])) {
		result[colorSetting] = { ...current[colorSetting], ...imported[colorSetting] };
	}
	if (imported[tokenSetting]) {
		const existing = current[tokenSetting] && typeof current[tokenSetting] === 'object' && !Array.isArray(current[tokenSetting]) ? current[tokenSetting] : {};
		const existingRules = Array.isArray(existing.textMateRules) ? existing.textMateRules : [];
		const incomingRules = Array.isArray(imported[tokenSetting].textMateRules) ? imported[tokenSetting].textMateRules : [];
		const rules = [...existingRules];
		for (const incoming of incomingRules) {
			const index = rules.findIndex(rule => rule?.scope === incoming.scope);
			if (index < 0) { rules.push(incoming); continue; }
			const previous = rules[index];
			rules[index] = {
				...previous,
				settings: { ...(previous?.settings && typeof previous.settings === 'object' ? previous.settings : {}), ...incoming.settings }
			};
		}
		result[tokenSetting] = { ...existing, ...imported[tokenSetting], textMateRules: rules };
	}
	return result;
}

function createProfileSnapshot(keys, current, target) {
	const settings = {};
	for (const key of keys) { settings[key] = { target, value: current[key] === undefined ? null : current[key] }; }
	return { settings };
}

module.exports = { createProfileSnapshot, mergeProfileSettings };
