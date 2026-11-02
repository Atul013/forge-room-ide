export type PlanArtifactInput = { request: string; plan: string; model: string; task?: string; createdAt?: string };

const maxPlanArtifactText = 100_000;
const maxMetadataText = 240;

function metadata(value: string) {
	return value.replace(/[\u0000-\u001f\u007f]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, maxMetadataText).replace(/[|\\]/g, '\\$&');
}

export function formatPlanArtifact(input: PlanArtifactInput): string | undefined {
	if (!input || typeof input.request !== 'string' || typeof input.plan !== 'string' || typeof input.model !== 'string' || !input.plan.trim() || input.request.length > maxPlanArtifactText || input.plan.length > maxPlanArtifactText) { return undefined; }
	const date = input.createdAt && Number.isFinite(Date.parse(input.createdAt)) ? new Date(input.createdAt).toISOString() : new Date().toISOString();
	const request = input.request.trim();
	const requestSection = request ? `## Original request\n\n${request.split(/\r?\n/).map(line => `> ${line}`).join('\n')}\n\n` : '';
	const task = input.task ? `| Room task | ${metadata(input.task)} |\n` : '';
	return [
		'# Forge Room implementation plan',
		'',
		'| Field | Value |',
		'| --- | --- |',
		`| Created | ${date} |`,
		`| Model | ${metadata(input.model) || 'Unknown'} |`,
		task.trimEnd(),
		'',
		requestSection.trimEnd(),
		'## Proposed plan',
		'',
		input.plan.trim(),
		''
	].filter((line, index, lines) => line !== '' || lines[index - 1] !== '').join('\n');
}
