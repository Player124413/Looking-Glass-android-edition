# Preliminary release assessment

Prepared 27 September 2026 for a UK-based developer considering a worldwide internet release. This is a research baseline, not a solicitor's opinion or clearance in every country.

## Practical conclusion

**Do not treat this archive or a Rust translation as freely redistributable.** There is no evidence here of permission to publish the original game data, proprietary code, or a faithful remake containing its protected expression. The more promising route to investigate is an independently implemented compatibility engine, distributed without game assets and requiring users to supply data they are entitled to use. That reduces what the project redistributes; it does not automatically make the project lawful.

Publication update, 1 October 2026: the project is releasing an experimental compatibility engine and a reviewed source snapshot at [skulitom/LookingGlass](https://github.com/skulitom/LookingGlass). Original game data and private research are excluded. The requested gameplay previews are identified separately from the MIT source license. The assessment below remains a research baseline, not legal clearance.

## Ownership and the archive

American McGee's official site stated in April 2023 that EA owned the Alice property and controlled licensing and re-releases. That is an ownership lead, not a current chain-of-title investigation or an answer about every individual component. [Creator's statement](https://americanmcgee.com/2023/04/12/so-whats-next-and-the-answers-to-your-questions/).

An Internet Archive download and the absence of a convenient retail edition do not establish permission from the rights holder. No redistribution licence was identified in the supplied metadata. The user's entitlement to this particular copy has not been established. Older Alice books being public domain does not make the game's distinct art, music, dialogue, level designs and software public domain.

## UK baseline

Copying and making works available online are restricted acts unless authorised or covered by an applicable exception. A free release, credit, a disclaimer or a change of programming language does not itself supply permission. [CDPA section 17](https://www.legislation.gov.uk/ukpga/1988/48/section/17), [section 20](https://www.legislation.gov.uk/ukpga/1988/48/section/20).

Section 50BA permits a lawful user to observe, study and test a program's functioning to discover underlying ideas and principles while carrying out acts they are entitled to perform. It is not a general permission to distribute the game's content. [Section 50BA](https://www.legislation.gov.uk/ukpga/1988/48/section/50BA).

Section 50B's decompilation exception is narrower: it concerns information necessary for an independent interoperable program, subject to conditions. It does not cover unrestricted decompilation, information already readily available, unnecessary disclosure, or use to create substantially similar protected expression. It is not a blanket licence to translate the entire executable into Rust. [Section 50B](https://www.legislation.gov.uk/ukpga/1988/48/section/50B).

The official XML versions of these four provisions were retrieved directly when the browser's HTML reader could not parse their XHTML. The engineering approach uses local format observations and newly written Rust; selected Ghidra outputs stay in `private/`. That is an engineering separation, not proof that all legal requirements have been met. The same developer has seen binary analysis, so this should not be advertised as a formally separated clean-room implementation.

## Worldwide access

UK location does not create a worldwide release permission. Copyright protection extends across many countries through international agreements; relevant rules, exceptions and remedies differ. Where a claim can be brought and which law applies depend on the facts, including distribution and hosting. It is neither useful nor accurate to promise compliance with every country's law from this initial review. [UK IPO guidance on copyright abroad](https://www.gov.uk/guidance/protecting-your-copyright-abroad).

A public release assessment should therefore cover UK development, the intended hosting/distribution arrangements, significant target markets and the host's copyright process. Before publishing a faithful compatibility project, have a UK IP solicitor review the provenance record, entitlement to the input copy, reverse-engineering scope, included material and branding.

## Proposed release boundary

| Material | Proposed treatment |
| --- | --- |
| Newly authored Rust, synthetic tests and documentation | Candidate for source release, after provenance/dependency review |
| Original PK3s, textures, music, models, levels, dialogue and binaries | Excluded; users supply their own authorised data |
| Decompiled code and original-derived analysis dumps | Local research only; excluded from source package |
| Gameplay publicity | Six explicitly reviewed README preview files; underlying game artwork is excluded from MIT |
| Brand names, logos and implication of endorsement | Use only appropriately reviewed descriptive identification; no endorsement claim |
| Third-party dependencies | Comply with each applicable licence and distribution conditions |

The GPL release of Quake III code is not a release of all games built using related technology. id Software explicitly distinguishes its source release from copyrighted game data. No Quake III implementation was copied into this prototype. [id Software's source-release notice](https://github.com/id-Software/Quake-III-Arena/blob/master/README.txt).

The source package is a local review artifact, not evidence that release is cleared. An authorised asset licence or a separately designed game with original assets would present different release options.
