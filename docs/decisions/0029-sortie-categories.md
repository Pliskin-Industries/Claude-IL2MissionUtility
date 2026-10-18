# 0029. Air Tasking sortie categories: a label in the template file, chosen by the user, and a pool filter

- **Status:** Accepted
- **Date:** 2026-10-18
- **Decided by:** the user ("Let's create categories and have the user select them"; "4 plus interdiction which can also target naval vessels. allow another tab that can add or customize these"; "Also filter the pool"; "Tab inside Air Tasking"; "Can we add a label into missions instead?" → "In the template file"). Two details by Claude, marked below.

## Context

The step 8 review found that the library's role chips guessed "Close support" from the F-51 model, because a template's orders cannot tell a strike from close support (both are ground attack). The app saves nothing between runs, so a category list kept only in the app would reset at every start.

## Decision

- **Categories:** Air alert, Strike, Close support, Armed recon, Interdiction. Interdiction may also target naval vessels; ship targets arrive with decision 0028's snap targets.
- **The label lives in the template file:** a line `Category: <name>` in the top-level group's `Desc`. Air Tasking reads it when it loads a sortie, so the category travels with the file and needs no settings file. A label naming a category not in the list adds that category.
- **Without a label:** a user template starts from its orders (an air AttackArea → Air alert, otherwise Strike). *(Claude)* The six built-ins start from plan §5's display roles (AirAlert → Air alert, HVAR → Strike, CloseSupport → Close support, ArmedRecon → Armed recon, Yak raid → Strike, Il-10 → Strike) through a table keyed by file, and their files are **not** edited: flown probe flights place the built-ins as shipped and must regenerate byte for byte (`handoff/P14-probe-results.md`, decisions 0018 to 0022), and whether to change the curated files is STATE item 3.
- **In Air Tasking:** each library card has a category combo; the centre panel has two tabs, **Pool** and **Categories**. Categories lists each category with how many library sorties use it, and has Add, Rename and Remove (a removed category's sorties go back to their default). These changes last for this run; a category is kept between runs by being in a file's label. "Escorted" stays an automatic extra chip when a lead flies a Cover order.
- **Filter:** the pool table has a category filter ("All categories" or one), and Auto-fill fills only the sorties of the filtered category.
- *(Claude)* **Template Builder** gets its Category field after the upstream merge, in upstream's `src/ui/builder.rs`, so the merge does not conflict twice. Until then a user writes the line into the group's Desc in the mission editor.
- Type change (its own commit): `AirSortie.category: String`, `AirPlan.categories: Vec<String>`.

## Rejected

- Role chips from the orders alone (exact, but cannot show Close support or Armed recon).
- Plan §5 names by file for the built-ins only, order-based for the rest.
- A settings file in AppData for the categories: the first thing the app would save between runs; the file label does the job.
- An 8th rail tab for the categories.

## Consequences

- Generate output does not change: the category is read, never written, in this step.
- The Template Builder field and Interdiction's ship targets are follow-ups after the upstream merge (with decision 0028).
