(function () {
  const state = {
    rules: [],
    filtered: [],
  };

  const searchInput = document.getElementById("searchInput");
  const profileFilter = document.getElementById("profileFilter");
  const standardFilter = document.getElementById("standardFilter");
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
    const standards = [...new Set(state.rules.map((rule) => rule.standard))].sort();

    for (const profile of profiles) {
      profileFilter.appendChild(new Option(profile, profile));
    }

    for (const standard of standards) {
      standardFilter.appendChild(new Option(standard, standard));
    }
  }

  function applyFilters() {
    const query = searchInput.value.trim().toLowerCase();
    const profile = profileFilter.value;
    const standard = standardFilter.value;

    state.filtered = state.rules.filter((rule) => {
      if (profile !== "all" && rule.profile !== profile) {
        return false;
      }
      if (standard !== "all" && rule.standard !== standard) {
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
      const row = document.createElement("tr");
      row.innerHTML = `
        <td class="rule-id-cell"><code>${escapeHtml(rule.id)}</code><br /><small>${escapeHtml(rule.title)}</small></td>
        <td>${escapeHtml(rule.profile)}</td>
        <td>${escapeHtml(rule.standard)}</td>
        <td>${escapeHtml(rule.section_number)} · ${escapeHtml(rule.section_name)}</td>
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

  searchInput.addEventListener("input", applyFilters);
  profileFilter.addEventListener("change", applyFilters);
  standardFilter.addEventListener("change", applyFilters);

  loadRules().catch((error) => {
    resultsMeta.textContent = String(error.message || error);
  });
})();
