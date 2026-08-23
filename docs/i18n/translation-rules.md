# Translation rules

English | [中文](translation-rules.zh.md)

These rules govern translation between the two sides of a documentation pair. Both languages carry equal authority ([README.md](README.md)); the side being changed is the source for that update. They bind humans and agents equally. The levels follow RFC 2119 usage: **MUST** and **MUST NOT** are review- or gate-blocking; **SHOULD** needs a stated reason to deviate; **MAY** is discretionary.

## Faithfulness

- The counterpart **MUST** say what the authored side says: no added behavior, prerequisites, warnings, version claims, or examples, and no dropped ones.
- The counterpart **SHOULD** read as natural technical writing in its own language, not a word-for-word gloss. Translate meaning, restructure sentences where target grammar requires it, and keep the author's register.
- Translate an idiom's idea when a literal rendering would be unnatural. Do not preserve a source-language metaphor at the cost of clarity.

## Voice

- Match the nearest human-approved sample in [style-samples.md](style-samples.md). English uses concise professional developer prose; Chinese uses precise institutional technical Chinese.
- Write as a native technical author restating the content while preserving every source clause. Fluency never justifies adding or dropping a requirement.
- Give sentences an explicit actor when the target language would otherwise hide responsibility. Prefer established engineering idiom over calques.
- Split long paragraphs by semantic unit. Paragraph boundaries may differ from the source when the pair's structural contract does not encode them.
- When translating into Chinese, use Chinese category nouns with a first-mention English annotation where useful. Literal directory, file, command, identifier, and API references stay code-formatted English.

## Structure preservation

The pairing verifier checks heading depths, fenced code blocks, table dimensions, list shape, and semantic links. The paired files **MUST** match one to one in:

- heading hierarchy, order, and level;
- list kind, numbering, item count, and order;
- table columns, row count, and row order;
- fenced code-block info strings and contents, byte-for-byte, including comments;
- inline code spans such as commands, flags, paths, keys, event names, API names, and version numbers;
- relative links and exact query or fragment suffixes. Links into the active bilingual corpus use the `.md` target on the English side and `.zh.md` on the Chinese side. External URLs, images, and pure in-page fragments stay unchanged. Link text is translated.

Keep one physical line per paragraph when the repository's Markdown convention requires it, exactly one trailing newline, and no accidental whitespace changes. The language switcher is the explicit cross-locale exception.

## Terminology

- Load [terminology.md](terminology.md) before translating. Every listed term **MUST** follow its row and its forbidden-rendering rules.
- For a Chinese target, an unlisted technical term **MAY** use an established rendering from a reliable Chinese-language engineering source. Without reliable precedent, keep it in English and add it to 「待定术语」 with evidence and a proposed rendering.
- For an English target, use the established English term. If no unambiguous equivalent exists, preserve the source term with the shortest necessary gloss and record it as pending.
- A decided term enters `terminology.md` in the same change or in a clearly linked follow-up. Do not invent an inline rendering that conflicts with the table.

## Chinese typography

These rules govern Chinese prose; the English side follows the repository's normal Markdown conventions:

- **MUST** put one half-width space between Chinese text and Latin words, and between Chinese text and numerals: `每个 plugin 注册 3 个 tool`。
- **MUST** use full-width Chinese punctuation in Chinese prose: `，。：；？！（）「」`. Half-width punctuation remains inside code spans, complete English quotations, and numbers such as `3.5` and `1,024`.
- Prefer colons, periods, commas, or parentheses over em dashes when they make the sentence clearer.
- Use 顿号（、） between parallel Chinese list items.
- **MUST NOT** use full-width digits or Latin letters. Use canonical casing for proper nouns and identifiers.
- Use `你`, not `您`, in instructions addressed to the reader.
- Keep English words inside code spans; do not add a translated duplicate after a literal path, command, flag, or identifier.
- Do not place whitespace before Chinese punctuation or after an opening bracket; do not place whitespace before a closing bracket.
- Keep list-item punctuation internally consistent. Avoid a comma at the end of a complete list item.

## Review

Before recording a pair as consistent, read the translation without looking at the source and then compare both sides clause by clause. Check omitted or added facts, actors, modality, negation, conditions, scope, numbers, code, links, terminology, punctuation, and first-use annotations. Record pending terms instead of silently deciding them in prose.

The sidecar records a mechanical confirmation of exact contents. It is not evidence that a reviewer performed or passed this semantic checklist.
