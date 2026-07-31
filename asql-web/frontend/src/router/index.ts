import { createRouter, createWebHistory } from 'vue-router'
import HomePage from '../views/HomePage.vue'
import ConnectionsPage from '../views/ConnectionsPage.vue'
import TableDataPage from '../views/TableDataPage.vue'
import TableListPage from '../views/TableListPage.vue'
import TableStructurePage from '../views/TableStructurePage.vue'
import QueryPage from '../views/QueryPage.vue'
import SettingsPage from '../views/SettingsPage.vue'
import CreateTablePage from '../views/CreateTablePage.vue'
import AlterTablePage from '../views/AlterTablePage.vue'
import AlterDatabasePage from '../views/AlterDatabasePage.vue'
import PrivilegesPage from '../views/PrivilegesPage.vue'
import PrivilegesListPage from '../views/PrivilegesListPage.vue'
import IndexesPage from '../views/IndexesPage.vue'
import ExportPage from '../views/ExportPage.vue'
import ImportPage from '../views/ImportPage.vue'
import DatabaseListPage from '../views/DatabaseListPage.vue'
import CreateDatabasePage from '../views/CreateDatabasePage.vue'
import InsertDataPage from '../views/InsertDataPage.vue'
import ProcessListPage from '../views/ProcessListPage.vue'
import VariablesPage from '../views/VariablesPage.vue'
import StatusPage from '../views/StatusPage.vue'
import NotFoundPage from '../views/NotFoundPage.vue'

const routes = [
  { path: '/', name: 'Home', component: HomePage },
  { path: '/connections', name: 'Connections', component: ConnectionsPage },
  {
    path: '/browse/:connection',
    name: 'DatabaseList',
    component: DatabaseListPage,
  },
  {
    path: '/browse/:connection/:db/:table/structure',
    name: 'TableStructure',
    component: TableStructurePage,
  },
  {
    path: '/browse/:connection/:db/:table/alter',
    name: 'AlterTable',
    component: AlterTablePage,
  },
  {
    path: '/browse/:connection/:db/:table/indexes',
    name: 'Indexes',
    component: IndexesPage,
  },
  {
    path: '/browse/:connection/:db/:table/insert',
    name: 'InsertData',
    component: InsertDataPage,
  },
  {
    path: '/browse/:connection/:db/:table',
    name: 'TableData',
    component: TableDataPage,
  },
  {
    path: '/browse/:connection/:db/alter',
    name: 'AlterDatabase',
    component: AlterDatabasePage,
  },
  {
    path: '/browse/:connection/:db/privileges',
    name: 'Privileges',
    component: PrivilegesPage,
  },
  {
    path: '/browse/:connection/:db',
    name: 'DatabaseBrowse',
    component: TableListPage,
  },
  {
    path: '/create-table/:connection/:db',
    name: 'CreateTable',
    component: CreateTablePage,
  },
  {
    path: '/create-database/:connection',
    name: 'CreateDatabase',
    component: CreateDatabasePage,
  },
  {
    path: '/query',
    name: 'Query',
    component: QueryPage,
  },
  {
    path: '/query/:connection',
    name: 'QueryConn',
    component: QueryPage,
  },
  {
    path: '/export/:connection/:db',
    name: 'Export',
    component: ExportPage,
  },
  {
    path: '/import/:connection/:db',
    name: 'Import',
    component: ImportPage,
  },
  {
    path: '/privileges/:connection',
    name: 'PrivilegesList',
    component: PrivilegesListPage,
  },
  {
    path: '/create-user/:connection',
    name: 'CreateUser',
    component: PrivilegesPage,
  },
  {
    path: '/process-list/:connection',
    name: 'ProcessList',
    component: ProcessListPage,
  },
  {
    path: '/variables/:connection',
    name: 'Variables',
    component: VariablesPage,
  },
  {
    path: '/status/:connection',
    name: 'Status',
    component: StatusPage,
  },
  { path: '/settings', name: 'Settings', component: SettingsPage },
  { path: '/:pathMatch(.*)*', name: 'NotFound', component: NotFoundPage },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

export default router
