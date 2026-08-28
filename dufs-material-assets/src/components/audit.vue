<template>
    <v-dialog v-model="visible" fullscreen transition="dialog-bottom-transition" scrollable>
        <v-card class="d-flex flex-column h-100 bg-background">
            <v-toolbar color="primary" density="comfortable">
                <v-icon icon="$mdiShieldAccount" class="ml-4 mr-2" />
                <v-toolbar-title class="text-h6 font-weight-bold">{{ t('titleAudit') }}</v-toolbar-title>
                <v-spacer></v-spacer>
                <v-btn icon="$mdiClose" variant="text" @click="close"></v-btn>
            </v-toolbar>

            <v-card-text class="pa-4 pa-md-6 flex-grow-1 overflow-y-auto">
                <!-- KPI Stats Cards -->
                <v-row class="mb-4" dense>
                    <v-col cols="12" sm="6" md="3">
                        <v-card variant="tonal" color="primary" class="pa-4 rounded-lg">
                            <div class="d-flex align-center justify-space-between">
                                <div>
                                    <div class="text-caption text-medium-emphasis">{{ t('auditTodayTotal') }}</div>
                                    <div class="text-h5 font-weight-bold mt-1">{{ stats.today_total }}</div>
                                </div>
                                <v-avatar color="primary" variant="flat" size="42">
                                    <v-icon icon="$mdiHistory" color="white"></v-icon>
                                </v-avatar>
                            </div>
                        </v-card>
                    </v-col>
                    <v-col cols="12" sm="6" md="3">
                        <v-card variant="tonal" color="success" class="pa-4 rounded-lg">
                            <div class="d-flex align-center justify-space-between">
                                <div>
                                    <div class="text-caption text-medium-emphasis">{{ t('auditTodayUpload') }}</div>
                                    <div class="text-h5 font-weight-bold mt-1">{{ formatBytes(stats.today_upload_bytes) }}</div>
                                </div>
                                <v-avatar color="success" variant="flat" size="42">
                                    <v-icon icon="$mdiTrayArrowUp" color="white"></v-icon>
                                </v-avatar>
                            </div>
                        </v-card>
                    </v-col>
                    <v-col cols="12" sm="6" md="3">
                        <v-card variant="tonal" color="info" class="pa-4 rounded-lg">
                            <div class="d-flex align-center justify-space-between">
                                <div>
                                    <div class="text-caption text-medium-emphasis">{{ t('auditTodayDownload') }}</div>
                                    <div class="text-h5 font-weight-bold mt-1">{{ formatBytes(stats.today_download_bytes) }}</div>
                                </div>
                                <v-avatar color="info" variant="flat" size="42">
                                    <v-icon icon="$mdiDownload" color="white"></v-icon>
                                </v-avatar>
                            </div>
                        </v-card>
                    </v-col>
                    <v-col cols="12" sm="6" md="3">
                        <v-card variant="tonal" color="error" class="pa-4 rounded-lg">
                            <div class="d-flex align-center justify-space-between">
                                <div>
                                    <div class="text-caption text-medium-emphasis">{{ t('auditTodayAlerts') }}</div>
                                    <div class="text-h5 font-weight-bold mt-1">{{ stats.today_alerts }}</div>
                                </div>
                                <v-avatar color="error" variant="flat" size="42">
                                    <v-icon icon="$mdiDeleteForever" color="white"></v-icon>
                                </v-avatar>
                            </div>
                        </v-card>
                    </v-col>
                </v-row>

                <!-- Filters & Search Toolbar -->
                <v-card class="pa-4 mb-4 rounded-lg" elevation="1">
                    <v-row dense align="center">
                        <v-col cols="12" sm="6" md="3">
                            <v-select
                                v-model="filter.action"
                                :items="actionOptions"
                                item-title="title"
                                item-value="value"
                                :label="t('auditFilterAction')"
                                density="compact"
                                variant="outlined"
                                hide-details
                                @update:model-value="applyFilter"
                            ></v-select>
                        </v-col>
                        <v-col cols="12" sm="6" md="2">
                            <v-select
                                v-model="filter.status"
                                :items="statusOptions"
                                item-title="title"
                                item-value="value"
                                :label="t('auditFilterStatus')"
                                density="compact"
                                variant="outlined"
                                hide-details
                                @update:model-value="applyFilter"
                            ></v-select>
                        </v-col>
                        <v-col cols="12" sm="6" md="2">
                            <v-select
                                v-model="filter.timeRange"
                                :items="timeOptions"
                                item-title="title"
                                item-value="value"
                                :label="t('auditFilterTime')"
                                density="compact"
                                variant="outlined"
                                hide-details
                                @update:model-value="applyFilter"
                            ></v-select>
                        </v-col>
                        <v-col cols="12" sm="6" md="3">
                            <v-text-field
                                v-model="filter.q"
                                :placeholder="t('auditFilterSearch')"
                                density="compact"
                                variant="outlined"
                                hide-details
                                prepend-inner-icon="$mdiMagnify"
                                clearable
                                @keydown.enter="applyFilter"
                                @click:clear="applyFilter"
                            ></v-text-field>
                        </v-col>
                        <v-col cols="12" md="2" class="d-flex justify-end ga-2">
                            <v-btn
                                color="primary"
                                variant="tonal"
                                prepend-icon="$mdiRefresh"
                                :loading="loading"
                                @click="refreshAll"
                            >
                                {{ t('auditRefresh') }}
                            </v-btn>
                            <v-btn
                                color="secondary"
                                variant="tonal"
                                prepend-icon="$mdiDownload"
                                @click="exportCsv"
                            >
                                {{ t('auditExportCsv') }}
                            </v-btn>
                        </v-col>
                    </v-row>
                </v-card>

                <!-- Audit Log Records Table -->
                <v-card class="rounded-lg overflow-hidden" elevation="1">
                    <v-progress-linear v-if="loading" indeterminate color="primary"></v-progress-linear>
                    <v-table hover density="comfortable">
                        <thead>
                            <tr class="bg-surface-variant text-medium-emphasis">
                                <th class="text-left py-3 font-weight-bold" style="width: 170px;">{{ t('auditColTime') }}</th>
                                <th class="text-left py-3 font-weight-bold" style="width: 120px;">{{ t('auditColUser') }}</th>
                                <th class="text-left py-3 font-weight-bold" style="width: 120px;">{{ t('auditColAction') }}</th>
                                <th class="text-left py-3 font-weight-bold">{{ t('auditColPath') }}</th>
                                <th class="text-left py-3 font-weight-bold" style="width: 130px;">{{ t('auditColIp') }}</th>
                                <th class="text-left py-3 font-weight-bold" style="width: 90px;">{{ t('auditColStatus') }}</th>
                                <th class="text-left py-3 font-weight-bold" style="width: 100px;">{{ t('auditColSize') }}</th>
                                <th class="text-center py-3 font-weight-bold" style="width: 70px;">{{ t('headerActions') }}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-if="!records.length && !loading">
                                <td colspan="8" class="text-center py-8 text-medium-emphasis">
                                    {{ t('toastEmpty') }}
                                </td>
                            </tr>
                            <tr
                                v-for="item in records"
                                :key="item.id"
                                class="cursor-pointer"
                                @click="openDetail(item)"
                            >
                                <td class="text-body-2">{{ formatTime(item.timestamp) }}</td>
                                <td>
                                    <v-chip size="small" variant="flat" :color="item.user ? 'primary' : 'default'">
                                        {{ item.user || t('auditAnonymousUser') }}
                                    </v-chip>
                                </td>
                                <td>
                                    <v-chip size="small" :color="getActionColor(item.action)" variant="flat" class="font-weight-medium">
                                        {{ item.action }}
                                    </v-chip>
                                </td>
                                <td class="text-body-2 font-weight-medium text-truncate" style="max-width: 320px;" :title="item.path">
                                    <span>{{ item.path }}</span>
                                    <span v-if="item.target_path" class="text-caption text-medium-emphasis ml-1">
                                        ➔ {{ item.target_path }}
                                    </span>
                                </td>
                                <td class="text-body-2 text-mono">{{ item.ip }}</td>
                                <td>
                                    <v-chip
                                        size="x-small"
                                        :color="item.status === 'SUCCESS' ? 'success' : 'error'"
                                        variant="tonal"
                                    >
                                        {{ item.status_code }}
                                    </v-chip>
                                </td>
                                <td class="text-body-2 text-medium-emphasis">{{ item.size ? formatBytes(item.size) : '-' }}</td>
                                <td class="text-center" @click.stop>
                                    <v-btn icon="$mdiFileSearch" size="x-small" variant="text" @click="openDetail(item)"></v-btn>
                                </td>
                            </tr>
                        </tbody>
                    </v-table>

                    <!-- Pagination Footer -->
                    <v-divider></v-divider>
                    <div class="d-flex align-center justify-space-between pa-3 px-4 flex-wrap ga-2">
                        <div class="text-caption text-medium-emphasis">
                            {{ t('headerSizeSubdirectoryItems', [totalRecords]) }}
                        </div>
                        <div class="d-flex align-center ga-4">
                            <div class="d-flex align-center ga-2">
                                <span class="text-caption text-medium-emphasis">Page size:</span>
                                <v-select
                                    v-model="pagination.pageSize"
                                    :items="[10, 20, 50, 100]"
                                    density="compact"
                                    variant="outlined"
                                    hide-details
                                    style="width: 85px;"
                                    @update:model-value="fetchRecords"
                                ></v-select>
                            </div>
                            <v-pagination
                                v-model="pagination.page"
                                :length="totalPages"
                                :total-visible="5"
                                density="compact"
                                @update:model-value="fetchRecords"
                            ></v-pagination>
                        </div>
                    </div>
                </v-card>
            </v-card-text>
        </v-card>

        <!-- Detail Dialog -->
        <v-dialog v-model="detailDialog" max-width="640" scrollable>
            <v-card v-if="selectedRecord" class="rounded-lg">
                <v-toolbar color="primary" density="compact">
                    <v-toolbar-title class="text-subtitle-1">{{ t('auditDetailTitle') }} #{{ selectedRecord.id }}</v-toolbar-title>
                    <v-spacer></v-spacer>
                    <v-btn icon="$mdiClose" variant="text" size="small" @click="detailDialog = false"></v-btn>
                </v-toolbar>
                <v-card-text class="pa-4">
                    <v-list density="compact" lines="two">
                        <v-list-item :title="t('auditColTime')" :subtitle="formatTime(selectedRecord.timestamp) + ' (' + new Date(selectedRecord.timestamp).toISOString() + ')'" />
                        <v-list-item :title="t('auditColUser')" :subtitle="selectedRecord.user || t('auditAnonymousUser')" />
                        <v-list-item :title="t('auditColAction')">
                            <template #subtitle>
                                <v-chip size="small" :color="getActionColor(selectedRecord.action)" class="mt-1">
                                    {{ selectedRecord.action }}
                                </v-chip>
                            </template>
                        </v-list-item>
                        <v-list-item :title="t('auditColPath')" :subtitle="selectedRecord.path" />
                        <v-list-item v-if="selectedRecord.target_path" :title="t('auditColTargetPath')" :subtitle="selectedRecord.target_path" />
                        <v-list-item :title="t('auditColIp')" :subtitle="selectedRecord.ip" />
                        <v-list-item :title="t('auditColStatus')" :subtitle="`${selectedRecord.status} (HTTP ${selectedRecord.status_code})`" />
                        <v-list-item v-if="selectedRecord.size" :title="t('auditColSize')" :subtitle="`${formatBytes(selectedRecord.size)} (${selectedRecord.size} bytes)`" />
                        <v-list-item v-if="selectedRecord.message" title="Message" :subtitle="selectedRecord.message" />
                    </v-list>
                </v-card-text>
                <v-card-actions class="pa-3 justify-end">
                    <v-btn color="primary" variant="tonal" @click="detailDialog = false">{{ t('dialogButtonConfirmText') }}</v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </v-dialog>
</template>

<script setup>
import { ref, reactive, computed, watch, onMounted } from 'vue';
import { useI18n } from 'petite-vue-i18n';
import { pathPrefix } from '../common.js';

const props = defineProps({
    modelValue: {
        type: Boolean,
        default: false,
    },
});

const emit = defineEmits(['update:modelValue']);

const { t } = useI18n();

const visible = computed({
    get: () => props.modelValue,
    set: (v) => emit('update:modelValue', v),
});

const loading = ref(false);
const records = ref([]);
const totalRecords = ref(0);
const detailDialog = ref(false);
const selectedRecord = ref(null);

const stats = reactive({
    today_total: 0,
    today_upload_bytes: 0,
    today_download_bytes: 0,
    today_alerts: 0,
});

const filter = reactive({
    action: 'all',
    status: 'all',
    timeRange: 'all',
    q: '',
});

const pagination = reactive({
    page: 1,
    pageSize: 20,
});

const totalPages = computed(() => Math.ceil(totalRecords.value / pagination.pageSize) || 1);

const actionOptions = computed(() => [
    { title: t('auditActionAll'), value: 'all' },
    { title: 'Upload (上传)', value: 'UPLOAD' },
    { title: 'Delete (删除)', value: 'DELETE' },
    { title: 'Move (移动)', value: 'MOVE' },
    { title: 'Copy (复制)', value: 'COPY' },
    { title: 'Mkdir (创建目录)', value: 'MKDIR' },
    { title: 'Download (下载)', value: 'DOWNLOAD' },
    { title: 'Zip Download (压缩包下载)', value: 'ZIP_DOWNLOAD' },
    { title: 'Edit Save (编辑保存)', value: 'EDIT_SAVE' },
    { title: 'Auth Fail (鉴权失败)', value: 'AUTH_FAIL' },
    { title: 'Login (登录)', value: 'LOGIN' },
]);

const statusOptions = computed(() => [
    { title: t('auditStatusAll'), value: 'all' },
    { title: t('auditStatusSuccess'), value: 'SUCCESS' },
    { title: t('auditStatusFailed'), value: 'FAILED' },
]);

const timeOptions = computed(() => [
    { title: t('auditFilterTimeAll'), value: 'all' },
    { title: t('auditFilterTime24h'), value: '24h' },
    { title: t('auditFilterTime7d'), value: '7d' },
    { title: t('auditFilterTime30d'), value: '30d' },
]);

function getActionColor(action) {
    switch (action) {
        case 'UPLOAD': return 'success';
        case 'DELETE': return 'error';
        case 'MOVE': return 'warning';
        case 'COPY': return 'cyan';
        case 'MKDIR': return 'teal';
        case 'DOWNLOAD': return 'info';
        case 'ZIP_DOWNLOAD': return 'deep-purple';
        case 'EDIT_SAVE': return 'purple';
        case 'AUTH_FAIL': return 'deep-orange';
        case 'LOGIN': return 'primary';
        default: return 'default';
    }
}

function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatTime(timestamp) {
    if (!timestamp) return '-';
    const date = new Date(timestamp);
    return date.toLocaleString();
}

async function fetchStats() {
    try {
        const prefix = pathPrefix.endsWith('/') ? pathPrefix : pathPrefix + '/';
        const res = await fetch(`${prefix}__dufs__/api/audit/stats`);
        if (res.ok) {
            const data = await res.json();
            stats.today_total = data.today_total || 0;
            stats.today_upload_bytes = data.today_upload_bytes || 0;
            stats.today_download_bytes = data.today_download_bytes || 0;
            stats.today_alerts = data.today_alerts || 0;
        }
    } catch (e) {
        console.error('Failed to fetch audit stats', e);
    }
}

async function fetchRecords() {
    loading.value = true;
    try {
        const prefix = pathPrefix.endsWith('/') ? pathPrefix : pathPrefix + '/';
        const params = new URLSearchParams();
        params.set('page', pagination.page.toString());
        params.set('page_size', pagination.pageSize.toString());

        if (filter.action && filter.action !== 'all') {
            params.set('action', filter.action);
        }
        if (filter.status && filter.status !== 'all') {
            params.set('status', filter.status);
        }
        if (filter.q && filter.q.trim()) {
            params.set('q', filter.q.trim());
        }
        if (filter.timeRange !== 'all') {
            const now = Date.now();
            let start = 0;
            if (filter.timeRange === '24h') start = now - 24 * 60 * 60 * 1000;
            else if (filter.timeRange === '7d') start = now - 7 * 24 * 60 * 60 * 1000;
            else if (filter.timeRange === '30d') start = now - 30 * 24 * 60 * 60 * 1000;
            if (start > 0) params.set('start_time', start.toString());
        }

        const res = await fetch(`${prefix}__dufs__/api/audit?${params.toString()}`);
        if (res.ok) {
            const data = await res.json();
            records.value = data.data || [];
            totalRecords.value = data.total || 0;
        }
    } catch (e) {
        console.error('Failed to fetch audit records', e);
    } finally {
        loading.value = false;
    }
}

function applyFilter() {
    pagination.page = 1;
    fetchRecords();
}

function refreshAll() {
    fetchStats();
    fetchRecords();
}

function exportCsv() {
    const prefix = pathPrefix.endsWith('/') ? pathPrefix : pathPrefix + '/';
    const params = new URLSearchParams();
    if (filter.action && filter.action !== 'all') params.set('action', filter.action);
    if (filter.status && filter.status !== 'all') params.set('status', filter.status);
    if (filter.q && filter.q.trim()) params.set('q', filter.q.trim());
    window.open(`${prefix}__dufs__/api/audit/export?${params.toString()}`, '_blank');
}

function openDetail(item) {
    selectedRecord.value = item;
    detailDialog.value = true;
}

function close() {
    visible.value = false;
}

watch(visible, (val) => {
    if (val) {
        refreshAll();
    }
});

onMounted(() => {
    if (visible.value) {
        refreshAll();
    }
});
</script>

<style scoped>
.text-mono {
    font-family: monospace;
}
.cursor-pointer {
    cursor: pointer;
}
</style>
