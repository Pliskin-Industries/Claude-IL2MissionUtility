# Correction to the step 3c prompt (MCU counts)

The step 3c prompt said "20 MCUs per wrapped HVAR copy (was 19), 19 with a template RTB timer (was 18)". That was the orchestrator's error: the shell's tests count the MCUs the shell **adds**, and keeping Zone IN does not add one; it removes one fewer. The correct statement, also in decision 0018's dated correction section:

- The shell adds 19 MCUs to a wrapped HVAR copy, plus one `RTB 1` waypoint (unchanged), and 18 with a template RTB timer (unchanged).
- It removes 8 template MCUs from the HVAR copy (was 9: Zone IN is no longer removed) and 7 with a template RTB timer (was 8).
- So each copy holds one MCU more than under decision 0017, and two check zones.

The count tests assert exactly that: added 19 / 18 (keep the names `shell_adds_19_mcus_per_copy` and `shell_keeps_template_rtb_timer`), removed 8 / 7, and the wrapped copy's total MCU count is one higher than the same copy wrapped without Zone IN would be (state the literal totals). Everything else in `handoff/P14-step-3c-codex-prompt.md` stands.
