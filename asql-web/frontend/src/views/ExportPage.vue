<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import SearchableSelect from "../components/SearchableSelect.vue";
import { api } from "../api";
import type { Connection } from "../types";

const { t } = useI18n();

const route = useRoute();

const connection = computed(() => route.params.connection as string);
const database = computed(() => route.params.db as string);

const connections = ref<Connection[]>([]);
const activeConn = ref("");
const activeDb = ref("");
const databases = ref<string[]>([]);
const tables = ref<{ name: string; rows: number; size: number }[]>([]);
const selectedTables = ref<Set<string>>(new Set());
const selectedData = ref<Set<string>>(new Set());

const output = ref<"open" | "save">("open");
const format = ref<"sql" | "csv" | "csv;" | "tsv">("sql");
const dbOption = ref<"skip" | "use" | "create" | "drop_create">("skip");
const tableOption = ref<"" | "DROP+CREATE" | "CREATE">("");

const exporting = ref(false);

function formatSize(bytes: number): string {
    if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(2) + " GB";
    if (bytes >= 1048576) return (bytes / 1048576).toFixed(2) + " MB";
    if (bytes >= 1024) return (bytes / 1024).toFixed(2) + " KB";
    return bytes + " B";
}

const selectedSummary = computed(() => {
    return tables.value
        .filter(t => selectedTables.value.has(t.name) || selectedData.value.has(t.name))
        .map(t => ({
            name: t.name,
            hasDDL: selectedTables.value.has(t.name),
            hasData: selectedData.value.has(t.name),
            rows: t.rows,
            size: t.size,
        }));
});

onMounted(async () => {
    try {
        connections.value = await api.listConnections();
        const connParam = route.params.connection as string;
        const dbParam = route.params.db as string;
        if (connParam) {
            activeConn.value = connParam;
            const dbResults = await api.listDatabases(connParam);
            databases.value = (dbResults.data || []).map(
                (r: any) => r.name || "",
            );
            if (dbParam) {
                activeDb.value = dbParam;
                await loadTables();
            } else if (databases.value.length > 0) {
                activeDb.value = databases.value[0];
                await loadTables();
            }
        }
    } catch {
        /* ignore */
    }
});

async function loadTables() {
    if (!activeConn.value || !activeDb.value) return;
    try {
        const result = await api.listTables(activeConn.value, activeDb.value);
        tables.value = (result.data || []).map((r: any) => ({
            name: r.table_name || "",
            rows: r.table_rows ?? 0,
            size: r.table_size ?? 0,
        }));
    } catch {
        /* ignore */
    }
}

function toggleSelectAll() {
    if (selectedTables.value.size === tables.value.length) {
        selectedTables.value = new Set();
    } else {
        selectedTables.value = new Set(tables.value.map((t) => t.name));
    }
}

function toggleTable(name: string) {
    const s = new Set(selectedTables.value);
    if (s.has(name)) s.delete(name);
    else s.add(name);
    selectedTables.value = s;
}

function toggleDataAll() {
    if (selectedData.value.size === tables.value.length) {
        selectedData.value = new Set();
    } else {
        selectedData.value = new Set(tables.value.map((t) => t.name));
    }
}

function toggleData(name: string) {
    const s = new Set(selectedData.value);
    if (s.has(name)) s.delete(name);
    else s.add(name);
    selectedData.value = s;
}

async function doExport() {
    const conn = activeConn.value;
    const db = activeDb.value;
    if (!conn) return;

    const allTables = new Set([...selectedTables.value, ...selectedData.value]);
    if (allTables.size === 0) return;

    exporting.value = true;

    const tableOptMap: Record<string, string> = {
        "DROP+CREATE": "drop_create",
        CREATE: "create",
        data: "skip",
    };

    const body = {
        method: output.value === "open" ? "open" : "save",
        database: db,
        db_option: dbOption.value,
        tables_all: false,
        tables: Array.from(allTables).map((name) => ({
            name,
            ddl: tableOption.value !== "" && selectedTables.value.has(name),
            data: selectedData.value.has(name),
        })),
        table_option: tableOptMap[tableOption.value] || "drop_create",
        data_format: format.value,
    };

    try {
        const res = await fetch(
            `/api/connections/${encodeURIComponent(conn)}/export`,
            {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(body),
            },
        );

        if (!res.ok) {
            const err = await res.json().catch(() => ({}));
            throw new Error(err.error || `HTTP ${res.status}`);
        }

        const blob = await res.blob();

        if (output.value === "save") {
            const url = URL.createObjectURL(blob);
            const disposition = res.headers.get("Content-Disposition");
            let filename = `export.${format.value}`;
            if (disposition) {
                const match = disposition.match(/filename="(.+)"/);
                if (match) filename = match[1];
            }
            const a = document.createElement("a");
            a.href = url;
            a.download = filename;
            a.click();
            URL.revokeObjectURL(url);
        } else {
            const text = await blob.text();
            const textBlob = new Blob([text], { type: "text/plain;charset=utf-8" });
            const textUrl = URL.createObjectURL(textBlob);
            window.open(textUrl);
        }
    } catch (e: any) {
        window.open("data:text/plain," + encodeURIComponent(`Error: ${e.message || e}`));
    }
    exporting.value = false;
}

</script>

<template>
    <div>
        <div class="page-header">
            {{ $t("export.title") }}: {{ database || activeDb }}
        </div>
        <div class="page-content">
            <table class="form-table" style="width: auto; margin-bottom: 12px">
                <tbody>
                    <tr>
                        <th>{{ $t("export.output") }}</th>
                        <td>
                            <label
                                ><input
                                    v-model="output"
                                    value="open"
                                    type="radio"
                                />
                                {{ $t("export.open") }}</label
                            >
                            <label style="margin-left: 8px"
                                ><input
                                    v-model="output"
                                    value="save"
                                    type="radio"
                                />
                                {{ $t("export.save") }}</label
                            >
                        </td>
                    </tr>
                    <tr>
                        <th>{{ $t("export.format") }}</th>
                        <td>
                            <label
                                ><input
                                    v-model="format"
                                    value="sql"
                                    type="radio"
                                />
                                {{ $t("export.sql") }}</label
                            >
                            <label style="margin-left: 8px"
                                ><input
                                    v-model="format"
                                    value="csv"
                                    type="radio"
                                />
                                {{ $t("export.csv") }}</label
                            >
                            <label style="margin-left: 8px"
                                ><input
                                    v-model="format"
                                    value="csv;"
                                    type="radio"
                                />
                                CSV;</label
                            >
                            <label style="margin-left: 8px"
                                ><input
                                    v-model="format"
                                    value="tsv"
                                    type="radio"
                                />
                                {{ $t("export.tsv") }}</label
                            >
                        </td>
                    </tr>
                    <tr>
                        <th>{{ $t("export.database") }}</th>
                        <td>
                            <SearchableSelect
                                v-model="dbOption"
                                :options="[
                                    { value: 'skip', label: '' },
                                    { value: 'use', label: 'USE' },
                                    { value: 'create', label: 'CREATE' },
                                    {
                                        value: 'drop_create',
                                        label: 'DROP+CREATE',
                                    },
                                ]"
                                style="width: 150px"
                            />
                        </td>
                    </tr>
                    <tr>
                        <th>{{ $t("export.tables") }}</th>
                        <td>
                            <SearchableSelect
                                v-model="tableOption"
                                :options="[
                                    { value: '', label: '' },
                                    {
                                        value: 'DROP+CREATE',
                                        label: 'DROP+CREATE',
                                    },
                                    { value: 'CREATE', label: 'CREATE' },
                                ]"
                                style="width: 150px"
                            />
                        </td>
                    </tr>
                </tbody>
            </table>

            <button
                @click="doExport"
                :disabled="exporting || selectedTables.size === 0"
            >
                {{ exporting ? "Exporting..." : $t("export.exportBtn") }}
            </button>

            <div
                v-if="selectedSummary.length"
                style="margin-top: 12px; padding: 8px; border: 1px solid #ccc; border-radius: 4px; background: #fafafa; font-size: 13px; max-height: 120px; overflow-y: auto"
            >
                <strong>{{ $t("export.selectedTables") }}:</strong>
                <div style="margin-top: 4px; display: flex; flex-wrap: wrap; gap: 6px">
                    <span
                        v-for="s in selectedSummary"
                        :key="s.name"
                        style="display: inline-block; padding: 2px 8px; border-radius: 3px"
                        :style="{
                            background: s.hasDDL && s.hasData ? '#d4edda' : s.hasDDL ? '#cce5ff' : '#fff3cd'
                        }"
                    >
                        {{ s.name }}
                        <span v-if="s.hasDDL && s.hasData">({{ $t("export.ddl") }} + {{ $t("export.dataOnly") }})</span>
                        <span v-else-if="s.hasDDL">({{ $t("export.ddl") }})</span>
                        <span v-else-if="s.hasData">({{ $t("export.dataOnly") }})</span>
                        <span style="color: #666; font-size: 11px; margin-left: 4px">
                            {{ s.rows.toLocaleString() }} rows, {{ formatSize(s.size) }}
                        </span>
                    </span>
                    <span v-if="!selectedSummary.length" style="color: #999">{{ $t("export.none") }}</span>
                </div>
            </div>

            <table style="margin-top: 12px">
                <thead>
                    <tr>
                        <th style="width: 30px">
                            <input
                                type="checkbox"
                                :checked="
                                    selectedTables.size === tables.length &&
                                    tables.length > 0
                                "
                                @change="toggleSelectAll"
                            />
                        </th>
                        <th>{{ $t("export.tables") }}</th>
                        <th style="width: 30px">
                            <input
                                type="checkbox"
                                :checked="
                                    selectedData.size === tables.length &&
                                    tables.length > 0
                                "
                                @change="toggleDataAll"
                            />
                        </th>
                        <th>{{ $t("export.data") }}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="t in tables" :key="t.name">
                        <td>
                            <input
                                type="checkbox"
                                :checked="selectedTables.has(t.name)"
                                @change="toggleTable(t.name)"
                            />
                        </td>
                        <td>{{ t.name }}</td>
                        <td>
                            <input
                                type="checkbox"
                                :checked="selectedData.has(t.name)"
                                @change="toggleData(t.name)"
                            />
                        </td>
                        <td class="num">{{ t.rows }}</td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>
</template>
