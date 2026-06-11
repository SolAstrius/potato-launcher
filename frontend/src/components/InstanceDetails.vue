<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Pencil, Trash2 } from 'lucide-vue-next';
import DeleteConfirmModal from './DeleteConfirmModal.vue';
import { apiService, formatError } from '@/services/api';
import type { AuthBackend, InstanceBase, InstanceResponse, IncludeRule } from '@/types/api';
import { AuthType, LoaderType } from '@/types/api';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useInstanceForm } from '@/composables/useInstanceForm';
import InstanceFormFields from '@/components/InstanceFormFields.vue';
import { useNotification } from '@/composables/useNotification';

const props = defineProps<{
  instance: InstanceResponse;
}>();

const emit = defineEmits<{
  (event: 'updated', payload: { name: string; data: Partial<InstanceResponse> }): void;
  (event: 'deleted', name: string): void;
}>();

const { showError } = useNotification();

type EditableFields = InstanceBase;

const toEditableFields = (instance: InstanceResponse): EditableFields => ({
  name: instance.name,
  packwiz_url: instance.packwiz_url,
  minecraft_version: instance.minecraft_version,
  loader_name: instance.loader_name,
  loader_version: instance.loader_version,
  recommended_xmx: instance.recommended_xmx,
  auth_backend: { ...instance.auth_backend },
  include: instance.include?.map(rule => ({ ...rule })) || [],
});

const isEditing = ref(false);
const showDeleteConfirm = ref(false);
const updating = ref(false);

const guard = computed(() => isEditing.value);

const {
  formData: editData,
  minecraftVersions,
  availableLoaders,
  loaderVersions,
  loadingMinecraftVersions,
  loadingLoaders,
  loadingLoaderVersions,
  handleInputChange: setFieldValue,
  handleAuthBackendChange: setAuthFieldValue,
  addIncludeRule,
  removeIncludeRule,
  updateIncludeRule,
  loadMinecraftVersions,
  resetFormData,
} = useInstanceForm({
  initialData: toEditableFields(props.instance),
  guard,
  mode: 'edit',
});

const setEditDataFromProps = () => {
  resetFormData(toEditableFields(props.instance));
};

watch(
  () => props.instance.name,
  () => {
    isEditing.value = false;
    showDeleteConfirm.value = false;
    minecraftVersions.value = [];
    availableLoaders.value = [];
    loaderVersions.value = [];
    setEditDataFromProps();
  },
);

const handleEdit = async () => {
  setEditDataFromProps();
  await loadMinecraftVersions();
  isEditing.value = true;
};

const handleCancel = () => {
  isEditing.value = false;
  showDeleteConfirm.value = false;
  availableLoaders.value = [];
  loaderVersions.value = [];
  setEditDataFromProps();
};

const handleUpdate = async () => {
  updating.value = true;
  try {
    let payload: InstanceBase;
    if (editData.packwiz_url?.trim()) {
      payload = {
        name: editData.name,
        packwiz_url: editData.packwiz_url.trim(),
        minecraft_version: '',
        loader_name: LoaderType.VANILLA,
        recommended_xmx: editData.recommended_xmx,
        auth_backend: { ...editData.auth_backend },
      };
    } else {
      payload = {
        ...editData,
        auth_backend: { ...editData.auth_backend },
        include: editData.include?.map(rule => ({ ...rule })),
      };
      if (payload.loader_name === LoaderType.VANILLA) {
        delete payload.loader_version;
      }
    }

    const updated = await apiService.updateInstance(props.instance.name, payload);
    emit('updated', { name: props.instance.name, data: updated });
    handleCancel();
  } catch (err) {
    const message = formatError(err, 'Failed to update instance');
    console.error(message, err);
    showError(message);
  } finally {
    updating.value = false;
  }
};

const handleDelete = () => {
  emit('deleted', props.instance.name);
  showDeleteConfirm.value = false;
};

const updateField = (field: keyof EditableFields, value: string | LoaderType) => {
  setFieldValue(field, value);
};

const updateAuthField = (field: keyof AuthBackend, value: string | AuthType) => {
  setAuthFieldValue(field, value);
};

const authTypeLabel = computed(() => editData.auth_backend.type);

const filebrowserUrl = computed(() => `/filebrowser/files/${props.instance.name}`);
</script>

<template>
  <div class="space-y-6 p-4">
    <Card>
      <CardHeader>
        <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <CardTitle>{{ isEditing ? 'Edit Instance' : props.instance.name }}</CardTitle>
            <CardDescription>
              {{
                isEditing
                  ? 'Update the configuration or upload new files.'
                  : 'Review the active configuration for this instance.'
              }}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent class="space-y-6">
        <template v-if="isEditing">
          <form class="space-y-5" @submit.prevent="handleUpdate">
            <InstanceFormFields id-prefix="edit" :form-data="editData" :minecraft-versions="minecraftVersions"
              :available-loaders="availableLoaders" :loader-versions="loaderVersions"
              :loading-minecraft-versions="loadingMinecraftVersions" :loading-loaders="loadingLoaders"
              :loading-loader-versions="loadingLoaderVersions" :disabled="updating" @update-field="updateField"
              @update-auth-field="updateAuthField" @add-include-rule="addIncludeRule"
              @remove-include-rule="removeIncludeRule" @update-include-rule="updateIncludeRule" />
            <div class="flex flex-wrap items-center justify-between gap-3">
              <Button type="button" :disabled="updating" @click="handleCancel">
                Cancel
              </Button>
              <Button variant="outline" type="button" as-child>
                <a :href="filebrowserUrl" target="_blank" rel="noopener noreferrer">
                  Manage instance files
                </a>
              </Button>
              <Button type="submit" :disabled="updating">
                <span v-if="updating">Saving...</span>
                <span v-else>Save Changes</span>
              </Button>
            </div>
          </form>
        </template>
        <template v-else>
          <dl class="grid gap-4 sm:grid-cols-2">
            <div v-if="props.instance.packwiz_url" class="sm:col-span-2">
              <dt class="text-sm">Packwiz Pack</dt>
              <dd class="text-sm font-medium wrap-break-word">{{ props.instance.packwiz_url }}</dd>
            </div>
            <template v-else>
              <div>
                <dt class="text-sm">Minecraft Version</dt>
                <dd class="text-sm font-medium">{{ props.instance.minecraft_version }}</dd>
              </div>
              <div>
                <dt class="text-sm">Mod Loader</dt>
                <dd class="text-sm font-medium capitalize">{{ props.instance.loader_name }}</dd>
              </div>
              <div>
                <dt class="text-sm">Loader Version</dt>
                <dd class="text-sm font-medium">{{ props.instance.loader_version }}</dd>
              </div>
            </template>
            <div>
              <dt class="text-sm">Authentication Type</dt>
              <dd class="text-sm font-medium capitalize">{{ authTypeLabel }}</dd>
            </div>
            <div v-if="props.instance.recommended_xmx">
              <dt class="text-sm">Recommended Xmx</dt>
              <dd class="text-sm font-medium">{{ props.instance.recommended_xmx }}</dd>
            </div>
            <div
              v-if="props.instance.auth_backend.type === AuthType.TELEGRAM && props.instance.auth_backend.auth_base_url"
              class="sm:col-span-2">
              <dt class="text-sm">Auth Base URL</dt>
              <dd class="text-sm font-medium wrap-break-word">{{ props.instance.auth_backend.auth_base_url }}</dd>
            </div>
            <template v-if="props.instance.auth_backend.type === AuthType.ELY_BY">
              <div>
                <dt class="text-sm">Client ID</dt>
                <dd class="text-sm font-medium wrap-break-word">{{ props.instance.auth_backend.client_id }}</dd>
              </div>
              <div>
                <dt class="text-sm">Client Secret</dt>
                <dd class="text-sm font-medium">••••••••••</dd>
              </div>
            </template>
            <div class="sm:col-span-2" v-if="props.instance.include && props.instance.include.length > 0">
              <dt class="text-sm mb-2">Include Rules</dt>
              <dd class="text-sm font-medium">
                <div class="border rounded-md divide-y">
                  <div v-for="(rule, index) in props.instance.include" :key="index"
                    class="p-3 flex items-start justify-between gap-4">
                    <div class="font-mono text-xs bg-muted px-1.5 py-0.5 rounded">{{ rule.path }}</div>
                    <div class="flex gap-2 text-xs text-muted-foreground">
                      <span v-if="rule.overwrite" class="text-primary font-medium">Overwrite</span>
                      <span v-if="rule.recursive" class="text-primary font-medium">Recursive</span>
                      <span v-if="rule.delete_extra" class="text-destructive font-medium">Delete Extra</span>
                    </div>
                  </div>
                </div>
              </dd>
            </div>
          </dl>
          <div class="flex flex-wrap items-center justify-between gap-3">
            <Button class="gap-2" @click="handleEdit">
              <Pencil class="h-4 w-4" />
              Update
            </Button>
            <Button variant="outline" type="button" as-child>
              <a :href="filebrowserUrl" target="_blank" rel="noopener noreferrer">
                Manage instance files
              </a>
            </Button>
            <Button v-if="!showDeleteConfirm" variant="destructive" class="gap-2"
              @click="showDeleteConfirm = true">
              <Trash2 class="h-4 w-4" />
              Delete
            </Button>
          </div>
        </template>
      </CardContent>
    </Card>
    <DeleteConfirmModal :is-open="showDeleteConfirm" :instance-name="props.instance.name" @confirm="handleDelete"
      @cancel="showDeleteConfirm = false" />
  </div>
</template>
