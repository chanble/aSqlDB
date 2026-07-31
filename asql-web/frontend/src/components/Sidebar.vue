<script setup lang="ts">
import { ref, computed, watch, onMounted, inject } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { api } from "../api";
import SearchableSelect from "../components/SearchableSelect.vue";
import type { Connection } from "../types";
import type { Ref } from "vue";

const { t } = useI18n();

const route = useRoute();
const router = useRouter();

const connections = ref<Connection[]>([]);
const activeConn = ref("");
const databases = ref<string[]>([]);
const activeDb = ref("");
const tables = ref<string[]>([]);
const activeTable = ref("");

function firstValue(row: Record<string, any>): string {
  return String(Object.values(row)[0] ?? '')
}

async function fetchDatabases(conn: string): Promise<string[]> {
  try {
    const result = await api.listDatabases(conn)
    return (result.data || []).map((r: any) => r.name || '')
  } catch {
    return []
  }
}

async function fetchTables(conn: string, db: string): Promise<string[]> {
  try {
    const result = await api.listTables(conn, db)
    return (result.data || []).map((r: any) => r.table_name || '')
  } catch {
    return []
  }
}

const isBrowsing = computed(() => !!route.params.connection);
const isTablePage = computed(() => !!route.params.table);

const activeConnInfo = computed(() =>
    connections.value.find((c) => c.name === activeConn.value),
);

onMounted(async () => {
    try {
        connections.value = await api.listConnections();
    } catch {
        /* ignore */
    }
});

watch(
    () => route.params,
    async (params: any) => {
        const conn = params.connection as string | undefined;
        const db = params.db as string | undefined;
        const tbl = params.table as string | undefined;
        console.log(
            "[watch] params:",
            JSON.stringify(params),
            "conn:",
            conn,
            "activeConn:",
            activeConn.value,
            "activeDb:",
            activeDb.value,
        );

        if (!conn) {
            console.log("[watch] no conn, resetting");
            activeConn.value = "";
            activeDb.value = "";
            activeTable.value = "";
            databases.value = [];
            tables.value = [];
            return;
        }

        if (conn && conn !== activeConn.value) {
            console.log("[watch] new conn detected:", conn);
            activeConn.value = conn;
            databases.value = [];
            activeDb.value = "";
            tables.value = [];
            activeTable.value = "";
            try {
                connections.value = await api.listConnections();
                databases.value = await fetchDatabases(conn);
            } catch {
                /* ignore */
            }
            if (!params.db) {
                await detectCurrentDb(conn);
            }
        }
        if (db && db !== activeDb.value) {
            activeDb.value = db;
            activeTable.value = "";
            tables.value = [];
            try {
                await api.useDatabase(activeConn.value, activeDb.value);
            } catch {
                /* ignore */
            }
            try {
                tables.value = await fetchTables(activeConn.value, activeDb.value);
            } catch {
                /* ignore */
            }
        }
        if (tbl) {
            activeTable.value = tbl;
        }
    },
    { immediate: true },
);

const dbListRefreshKey = inject<Ref<number>>("dbListRefreshKey", ref(0));
watch(dbListRefreshKey, async () => {
    if (activeConn.value) {
        try {
            databases.value = await fetchDatabases(activeConn.value);
        } catch {
            /* ignore */
        }
    }
});

const tableListRefreshKey = inject<Ref<number>>("tableListRefreshKey", ref(0));
watch(tableListRefreshKey, async () => {
    if (activeConn.value && activeDb.value) {
        try {
            tables.value = await fetchTables(activeConn.value, activeDb.value);
        } catch {
            /* ignore */
        }
    }
});

async function detectCurrentDb(conn: string) {
    try {
        const result = await api.currentDatabase(conn)
        const dbName = result.data
        if (dbName) {
            activeDb.value = dbName
            tables.value = []
            tables.value = await fetchTables(conn, activeDb.value)
        }
    } catch (e) {
        console.error("[detectCurrentDb] error:", e);
    }
}

async function selectConn(conn: string) {
    if (conn === activeConn.value) return;
    try {
        await api.testConnection(conn);
    } catch (e: any) {
        alert(t('sidebar.cannotConnect', { msg: e.message || e }));
        return;
    }
    activeConn.value = conn;
    activeDb.value = "";
    activeTable.value = "";
    databases.value = [];
    tables.value = [];
    fetchDatabases(conn)
        .then((dbs) => {
            databases.value = dbs;
        })
        .catch(() => {});
    router.push(`/browse/${encodeURIComponent(conn)}`);
}

async function selectDb(db: string) {
    console.log(
        "\[selectDb\] called with:",
        db,
        "activeConn:",
        activeConn.value,
    );
    activeDb.value = db;
    activeTable.value = "";
    tables.value = [];
    await api.useDatabase(activeConn.value, db).catch((e) => {
        console.log("[selectDb] USE failed:", e);
    });
    try {
        tables.value = await fetchTables(activeConn.value, activeDb.value);
    } catch {
        /* ignore */
    }
    if (route.path.startsWith("/browse/")) {
        router.push(
            `/browse/${encodeURIComponent(activeConn.value)}/${encodeURIComponent(db)}`,
        );
    }
}

function selectTable(tbl: string) {
    activeTable.value = tbl;
    router.push(
        `/browse/${encodeURIComponent(activeConn.value)}/${encodeURIComponent(activeDb.value)}/${encodeURIComponent(tbl)}`,
    );
}

function goStructure() {
    router.push(
        `/browse/${encodeURIComponent(activeConn.value)}/${encodeURIComponent(activeDb.value)}/${encodeURIComponent(activeTable.value)}/structure`,
    );
}

function goSql() {
    router.push(`/query/${encodeURIComponent(activeConn.value)}`);
}

function goExport() {
    router.push(
        `/export/${encodeURIComponent(activeConn.value)}/${encodeURIComponent(activeDb.value)}`,
    );
}

function goImport() {
    router.push(
        `/import/${encodeURIComponent(activeConn.value)}/${encodeURIComponent(activeDb.value)}`,
    );
}

function goCreateTable() {
    router.push(
        `/create-table/${encodeURIComponent(activeConn.value)}/${encodeURIComponent(activeDb.value)}`,
    );
}

function goAlterTable() {
    router.push(
        `/browse/${encodeURIComponent(activeConn.value)}/${encodeURIComponent(activeDb.value)}/${encodeURIComponent(activeTable.value)}/alter`,
    );
}

function editConn(c: Connection) {
    router.push(
        `/?edit=${encodeURIComponent(c.name)}&url=${encodeURIComponent(c.url)}&db_type=${encodeURIComponent(c.db_type)}`,
    );
}

async function deleteConn(c: Connection) {
    if (!confirm(t('sidebar.confirmDelete', { name: c.name }))) return;
    try {
        await api.removeConnection(c.name);
        connections.value = connections.value.filter(
            (conn) => conn.name !== c.name,
        );
    } catch (e: any) {
        alert(e.message || t('sidebar.deleteFailed'));
    }
}

function dbTypeIcon(dt: string | undefined): string {
    const t = (dt || "").toUpperCase();
    if (t.includes("MYSQL")) return "mysql";
    if (t.includes("POSTGRESQL") || t.includes("PG")) return "postgresql";
    if (t.includes("SQLITE")) return "sqlite";
    if (t.includes("SQLSERVER") || t.includes("MSSQL"))
        return "microsoft-sql-server";
    if (t.includes("ORACLE")) return "oracle";
    if (t.includes("MONGO")) return "mongodb";
    return "database";
}

function truncate(url: string, max = 30): string {
    return url.length > max ? url.slice(0, max) + "..." : url;
}
</script>

<template>
    <aside class="sidebar">
        <div class="sidebar-brand" @click="router.push('/')">
            <span class="logo">{{ $t('sidebar.brand') }}</span>
            <span class="version">{{ $t('sidebar.version') }}</span>
        </div>

        <!-- Connection list when not browsing -->
        <template v-if="!isBrowsing">
            <div class="sidebar-section">
                <div
                    v-for="c in connections"
                    :key="c.name"
                    :class="[
                        'conn-item',
                        { 'is-active': activeConn === c.name },
                    ]"
                >
                    <div class="conn-item-header">
                        <span class="conn-clickable" @click="selectConn(c.name)">
                            <span class="conn-name">[{{ c.db_type }}] {{ c.name }}</span>
                        </span>
                    </div>
                    <!-- <div class="conn-url" :title="c.url">{{ truncate(c.url) }}</div> -->
                    <div class="conn-actions">
                        <a @click.stop="editConn(c)" title="Edit">{{ $t('sidebar.edit') }}</a>
                        <a @click.stop="deleteConn(c)" title="Delete">{{ $t('sidebar.delete') }}</a>
                    </div>
                </div>
                <div v-if="!connections.length" class="empty-msg">
                    <em>{{ $t('sidebar.noConnections') }}</em>
                </div>
            </div>
        </template>

        <!-- Database browser when browsing -->
        <template v-else>
            <div class="db-selector">
                <label>{{ $t('sidebar.db') }}</label>
                <SearchableSelect
                    v-model="activeDb"
                    :options="[
                        { value: '', label: $t('sidebar.selectDatabase') },
                        ...databases.map((d) => ({ value: d, label: d })),
                    ]"
                    @change="selectDb(activeDb)"
                    size="small"
                />
            </div>

            <div class="sidebar-nav" v-if="activeDb">
                <a href="#" @click.prevent="goSql">{{ $t('sidebar.sqlCommand') }}</a>
                <a href="#" @click.prevent="goExport">{{ $t('sidebar.export') }}</a>
                <a href="#" @click.prevent="goImport">{{ $t('sidebar.import') }}</a>
                <a href="#" @click.prevent="goCreateTable">{{ $t('sidebar.createTable') }}</a>
            </div>

            <div class="table-list" v-if="activeDb">
                <div
                    v-for="t in tables"
                    :key="t"
                    :class="['table-item', { active: activeTable === t }]"
                >
                    <span class="table-clickable" @click="selectTable(t)">{{ t }}</span>
                </div>
            </div>

            <!-- Connection-level nav (no database selected) -->
            <div class="sidebar-nav" v-if="!activeDb">
                <a href="#" @click.prevent="goSql">{{ $t('sidebar.sqlCommand') }}</a>
            </div>
        </template>
    </aside>
</template>

<style scoped>
.sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
}

.sidebar-brand {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 12px 16px;
    border-bottom: 1px solid #999;
    cursor: pointer;
}

.sidebar-brand:hover {
    background: #ddeeff;
}

.sidebar-brand .logo {
    font-style: italic;
    font-size: 18px;
    color: #666;
}

.sidebar-brand .version {
    font-size: 12px;
    color: #999;
}

.sidebar-section {
    flex: 1;
    overflow-y: auto;
}

.conn-item {
    padding: 6px 12px;
    border-bottom: 1px solid #ddd;
}

.conn-item.is-active {
    background: #ddeeff;
}

.conn-item-header {
    display: flex;
    align-items: center;
    gap: 4px;
}

.conn-clickable {
    cursor: pointer;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.conn-clickable:hover {
    text-decoration: underline;
}

.conn-name {
    font-weight: 600;
    font-size: 13px;
}

.conn-url {
    font-size: 11px;
    color: #777;
    margin-left: 20px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.conn-actions {
    display: flex;
    gap: 8px;
    margin-left: 20px;
    margin-top: 2px;
    font-size: 11px;
}

.conn-actions a {
    color: #00f;
    cursor: pointer;
}

.conn-actions a:hover {
    color: red;
    text-decoration: underline;
}

.empty-msg {
    padding: 16px;
    font-size: 13px;
    color: #999;
    text-align: center;
}

.db-selector {
    padding: 8px 12px;
    border-bottom: 1px solid #999;
}

.db-selector label {
    font-size: 12px;
    font-weight: bold;
    margin-right: 4px;
}

.db-selector select {
    font-size: 12px;
    padding: 1px 4px;
    border: 1px solid #999;
    max-width: 100%;
}

.sidebar-nav {
    padding: 8px 12px;
    border-bottom: 1px solid #999;
}

.sidebar-nav a {
    display: inline-block;
    margin-right: 12px;
    font-size: 13px;
}

.table-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
}

.table-item {
    padding: 2px 12px 2px 20px;
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.table-item.active {
    background: #ddeeff;
    font-weight: bold;
}

.table-clickable {
    color: #00f;
    cursor: pointer;
}

.table-clickable:hover {
    background: #ddeeff;
    text-decoration: underline;
}

.table-item.active .table-clickable {
    font-weight: bold;
}
</style>
