const test = require('node:test');
const assert = require('node:assert/strict');
const { parseAgentSkill } = require('../dist/agentSkills');

test('parses required Agent Skills metadata and Markdown body', () => {
	const result = parseAgentSkill('---\nname: code-review\ndescription: Review code for correctness\n---\n\nCheck edge cases.', 'code-review');
	assert.deepEqual(result, { metadata: { name: 'code-review', description: 'Review code for correctness' }, body: 'Check edge cases.' });
});

test('parses folded and literal YAML description blocks', () => {
	const folded = parseAgentSkill('---\nname: code-review\ndescription: >-\n  Review correctness\n  and edge cases\n---\nRead the diff.', 'code-review');
	assert.equal(folded.metadata.description, 'Review correctness and edge cases');
	const literal = parseAgentSkill('---\nname: code-review\ndescription: |\n  Review correctness\n  and edge cases\n---\nRead the diff.', 'code-review');
	assert.equal(literal.metadata.description, 'Review correctness\nand edge cases');
});

test('requires valid frontmatter and a directory-matching name', () => {
	assert.match(parseAgentSkill('# not a skill', 'review').error, /frontmatter/);
	assert.match(parseAgentSkill('---\nname: Review\ndescription: x\n---\nbody', 'Review').error, /lowercase/);
	assert.match(parseAgentSkill('---\nname: review\ndescription: x\n---\nbody', 'other').error, /parent directory/);
});

test('rejects duplicate fields, missing description, oversized files, and empty body', () => {
	assert.match(parseAgentSkill('---\nname: review\nname: review\ndescription: x\n---\nbody', 'review').error, /Duplicate/);
	assert.match(parseAgentSkill('---\nname: review\n---\nbody', 'review').error, /description/);
	assert.match(parseAgentSkill('x'.repeat(64_001), 'review').error, /64 KB/);
	assert.match(parseAgentSkill('---\nname: review\ndescription: x\n---\n', 'review').error, /empty/);
});
