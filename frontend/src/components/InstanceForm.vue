<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { apiService } from '@/services/api';
import type { AuthBackend, InstanceBase, IncludeRule } from '@/types/api';
import { AuthType, LoaderType } from '@/types/api';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useInstanceForm } from '@/composables/useInstanceForm';
import InstanceFormFields from '@/components/InstanceFormFields.vue';
import { formatError } from '@/services/api';
import { useNotification } from '@/composables/useNotification';

const { showError } = useNotification();

const emit = defineEmits<{
  (event: 'submitted', payload: InstanceBase): void;
}>();

const {
  formData,
  minecraftVersions,
  availableLoaders,
  loaderVersions,
  loadingMinecraftVersions,
  loadingLoaders,
  loadingLoaderVersions,
  handleInputChange,
  handleAuthBackendChange,
  addIncludeRule,
  removeIncludeRule,
  updateIncludeRule,
  loadMinecraftVersions,
  resetFormData,
} = useInstanceForm({ mode: 'create' });

const loading = ref(false);
const errors = reactive<Record<string, string>>({});

const isPackwiz = () => !!formData.packwiz_url?.trim();

const validate = () => {
  const newErrors: Record<string, string> = {};
  if (!formData.name.trim()) newErrors.name = 'Name is required';

  // Packwiz instances pull version/loader/mods from the pack; only auth still applies.
  if (!isPackwiz()) {
    if (!formData.minecraft_version) newErrors.minecraft_version = 'Minecraft version is required';
    if (!formData.loader_name) newErrors.loader_name = 'Loader is required';
    if (formData.loader_name !== LoaderType.VANILLA && !formData.loader_version) {
      newErrors.loader_version = 'Loader version is required';
    }
  }
  if (!formData.auth_backend.type) newErrors.auth_type = 'Authentication type is required';

  if (formData.auth_backend.type === AuthType.TELEGRAM && !formData.auth_backend.auth_base_url?.trim()) {
    newErrors.auth_base_url = 'Auth base URL is required for Telegram';
  }

  if (formData.auth_backend.type === AuthType.ELY_BY) {
    if (!formData.auth_backend.client_id?.trim()) {
      newErrors.client_id = 'Client ID is required for Ely.by';
    }
    if (!formData.auth_backend.client_secret?.trim()) {
      newErrors.client_secret = 'Client Secret is required for Ely.by';
    }
  }

  Object.keys(errors).forEach((key) => delete errors[key]);
  Object.assign(errors, newErrors);

  return Object.keys(newErrors).length === 0;
};

const resetForm = () => {
  resetFormData();
};

const handleSubmit = async () => {
  if (!validate()) {
    return;
  }

  try {
    loading.value = true;
    let payload: InstanceBase;
    if (isPackwiz()) {
      // Send only what a packwiz instance needs; the backend ignores the rest anyway.
      payload = {
        name: formData.name,
        packwiz_url: formData.packwiz_url?.trim(),
        minecraft_version: '',
        loader_name: LoaderType.VANILLA,
        recommended_xmx: formData.recommended_xmx,
        auth_backend: { ...formData.auth_backend },
      };
    } else {
      payload = {
        ...formData,
        auth_backend: { ...formData.auth_backend },
        include: formData.include?.map(rule => ({ ...rule })),
      };
      if (payload.loader_name === LoaderType.VANILLA) {
        delete payload.loader_version;
      }
    }

    await apiService.createInstance(payload);
    emit('submitted', payload);
    resetForm();
  } catch (err) {
    const message = formatError(err, 'Failed to create instance');
    console.error(message, err);
    showError(message);
  } finally {
    loading.value = false;
  }
};

const updateField = (field: keyof InstanceBase, value: string | LoaderType) => {
  handleInputChange(field, value);
  if (errors[field as string]) {
    delete errors[field as string];
  }
};

const updateAuthField = (field: keyof AuthBackend, value: string | AuthType) => {
  handleAuthBackendChange(field, value);
  const errorKey = field === 'type' ? 'auth_kind' : (field as string);
  if (errors[errorKey]) {
    delete errors[errorKey];
  }
};

onMounted(() => {
  loadMinecraftVersions().catch((err) => console.error(err));
});
</script>

<template>
  <div class="p-4">
    <Card>
      <CardHeader>
        <CardTitle>Create New Instance</CardTitle>
        <CardDescription>Create a new instance for the launcher.</CardDescription>
      </CardHeader>
      <CardContent>
        <form class="space-y-5" @submit.prevent="handleSubmit">
          <InstanceFormFields id-prefix="create" :form-data="formData" :errors="errors"
            :minecraft-versions="minecraftVersions" :available-loaders="availableLoaders"
            :loader-versions="loaderVersions" :loading-minecraft-versions="loadingMinecraftVersions"
            :loading-loaders="loadingLoaders" :loading-loader-versions="loadingLoaderVersions"
            @update-field="updateField" @update-auth-field="updateAuthField" @add-include-rule="addIncludeRule"
            @remove-include-rule="removeIncludeRule" @update-include-rule="updateIncludeRule" />
          <div>
            <Button type="submit" class="w-full" :disabled="loading">
              <span v-if="loading">Creating...</span>
              <span v-else>Create Instance</span>
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  </div>
</template>
