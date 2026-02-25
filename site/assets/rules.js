(function () {
  const state = {
    rules: [],
    filtered: [],
  };

  const searchInput = document.getElementById("searchInput");
  const profileFilter = document.getElementById("profileFilter");
  const resultsMeta = document.getElementById("resultsMeta");
  const rulesBody = document.getElementById("rulesBody");

  const totalRules = document.getElementById("totalRules");
  const asdRules = document.getElementById("asdRules");
  const otherRules = document.getElementById("otherRules");

  async function loadRules() {
    const response = await fetch("./data/rule_index.json", { cache: "no-store" });
    if (!response.ok) {
      throw new Error(`failed to load rule index (${response.status})`);
    }

    const payload = await response.json();
    const rules = Array.isArray(payload.rules) ? payload.rules : [];
    rules.sort((a, b) => String(a.id).localeCompare(String(b.id)));
    state.rules = rules;

    renderSummary();
    hydrateFilters();
    applyFilters();
  }

  function renderSummary() {
    const asdCount = state.rules.filter((rule) => rule.profile === "asd-ste100").length;
    const otherCount = state.rules.length - asdCount;

    totalRules.textContent = String(state.rules.length);
    asdRules.textContent = String(asdCount);
    otherRules.textContent = String(otherCount);
  }

  function hydrateFilters() {
    const profiles = [...new Set(state.rules.map((rule) => rule.profile))].sort();

    for (const profile of profiles) {
      profileFilter.appendChild(new Option(profile, profile));
    }
  }

  function applyFilters() {
    const query = searchInput.value.trim().toLowerCase();
    const profile = profileFilter.value;

    state.filtered = state.rules.filter((rule) => {
      if (profile !== "all" && rule.profile !== profile) {
        return false;
      }
      if (!query) {
        return true;
      }

      const haystack = [
        rule.id,
        rule.title,
        rule.standard,
        rule.section_name,
        rule.section_number,
        rule.rule_number,
        rule.citation,
      ]
        .join(" ")
        .toLowerCase();

      return haystack.includes(query);
    });

    renderTable();
  }

  function renderTable() {
    rulesBody.innerHTML = "";

    for (const rule of state.filtered) {
      const standardLabel = formatStandardLabel(rule.standard);
      const row = document.createElement("tr");
      row.innerHTML = `
        <td class="rule-id-cell"><code>${escapeHtml(rule.id)}</code><br /><small>${escapeHtml(rule.title)}</small></td>
        <td class="profile-cell">${escapeHtml(rule.profile)}</td>
        <td class="standard-cell">${escapeHtml(standardLabel)}</td>
        <td>${escapeHtml(rule.rule_number)}</td>
        <td>${escapeHtml(rule.citation)}</td>
      `;
      rulesBody.appendChild(row);
    }

    resultsMeta.textContent = `${state.filtered.length}/${state.rules.length} rules shown`;
  }

  function escapeHtml(value) {
    return String(value ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  // Keep public-facing standard names concise while preserving canonical JSON values.
  function formatStandardLabel(standard) {
    if (standard === "Project Glossary Policy") {
      return "Project Glossary";
    }
    return standard;
  }

  searchInput.addEventListener("input", applyFilters);
  profileFilter.addEventListener("change", applyFilters);

  loadRules().catch((error) => {
    resultsMeta.textContent = String(error.message || error);
  });
})();
