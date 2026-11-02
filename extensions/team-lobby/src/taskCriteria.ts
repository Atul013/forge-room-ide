export const maxAcceptanceCriteriaCount = 8;
export const maxAcceptanceCriterionLength = 180;
export const maxAcceptanceCriteriaTotal = 1_200;

export function isAcceptanceCriteria(value: unknown): value is string[] {
	return Array.isArray(value)
		&& value.length <= maxAcceptanceCriteriaCount
		&& value.every(item => typeof item === 'string' && item.trim().length > 0 && item.length <= maxAcceptanceCriterionLength)
		&& value.join('\n').length <= maxAcceptanceCriteriaTotal;
}

export function parseAcceptanceCriteria(input: string) {
	const criteria = input.split('|').map(item => item.trim()).filter(Boolean);
	if (criteria.length > maxAcceptanceCriteriaCount) { return { criteria, error: `Use up to ${maxAcceptanceCriteriaCount} criteria, separated by |.` }; }
	if (criteria.some(item => item.length > maxAcceptanceCriterionLength)) { return { criteria, error: `Each criterion must be ${maxAcceptanceCriterionLength} characters or fewer.` }; }
	if (new Set(criteria).size !== criteria.length) { return { criteria, error: 'Each acceptance criterion must be unique.' }; }
	if (criteria.join('\n').length > maxAcceptanceCriteriaTotal) { return { criteria, error: `Keep all criteria under ${maxAcceptanceCriteriaTotal} characters.` }; }
	return { criteria };
}
