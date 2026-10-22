'use strict';

async function confirmThemePreview(preview, confirm, restore) {
	let keep;
	try {
		await preview();
		keep = await confirm();
	} catch (error) {
		await restore();
		throw error;
	}
	if (!keep) { await restore(); }
	return keep === true;
}

module.exports = { confirmThemePreview };
