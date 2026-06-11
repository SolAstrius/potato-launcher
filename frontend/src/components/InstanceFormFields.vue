<script setup lang="ts">
import { computed } from 'vue';
import { Plus, Trash2 } from 'lucide-vue-next';
import type { AuthBackend, InstanceBase, IncludeRule } from '@/types/api';
import { AuthType, LoaderType } from '@/types/api';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Card, CardContent } from '@/components/ui/card';

const props = withDefaults(
    defineProps<{
        formData: InstanceBase;
        minecraftVersions: string[];
        availableLoaders: string[];
        loaderVersions: string[];
        loadingMinecraftVersions?: boolean;
        loadingLoaders?: boolean;
        loadingLoaderVersions?: boolean;
        errors?: Record<string, string>;
        disabled?: boolean;
        idPrefix?: string;
    }>(),
    {
        loadingMinecraftVersions: false,
        loadingLoaders: false,
        loadingLoaderVersions: false,
        errors: () => ({}),
        disabled: false,
        idPrefix: 'instance',
    },
);

const emit = defineEmits<{
    (event: 'update-field', field: keyof InstanceBase, value: string | LoaderType): void;
    (event: 'update-auth-field', field: keyof AuthBackend, value: string | AuthType): void;
    (event: 'add-include-rule'): void;
    (event: 'remove-include-rule', index: number): void;
    (event: 'update-include-rule', index: number, field: keyof IncludeRule, value: string | boolean): void;
}>();

const isVanillaLoader = computed(() => props.formData.loader_name === LoaderType.VANILLA);
const isPackwiz = computed(() => !!props.formData.packwiz_url?.trim());
</script>

<template>
    <div class="space-y-6">
        <div class="space-y-5">
            <div class="grid gap-4 sm:grid-cols-2">
                <div class="space-y-2 sm:col-span-2">
                    <Label :for="`${props.idPrefix}-name`">Instance Name *</Label>
                    <Input :id="`${props.idPrefix}-name`" :model-value="props.formData.name" :disabled="props.disabled"
                        placeholder="Enter instance name"
                        @update:modelValue="(value) => emit('update-field', 'name', value?.toString() ?? '')" />
                    <p v-if="props.errors?.name" class="text-sm text-destructive">
                        {{ props.errors.name }}
                    </p>
                </div>
                <div class="space-y-2 sm:col-span-2">
                    <Label :for="`${props.idPrefix}-packwiz-url`">Packwiz pack URL</Label>
                    <Input :id="`${props.idPrefix}-packwiz-url`" type="url"
                        :model-value="props.formData.packwiz_url || ''" :disabled="props.disabled"
                        placeholder="https://your.server/pack/pack.toml"
                        @update:modelValue="(value) => emit('update-field', 'packwiz_url', value?.toString() ?? '')" />
                    <p v-if="props.errors?.packwiz_url" class="text-sm text-destructive">
                        {{ props.errors.packwiz_url }}
                    </p>
                    <p v-else class="text-sm text-muted-foreground">
                        Optional. If set, the client builds this instance from the packwiz pack
                        (Minecraft version, loader, mods and configs come from the pack) and keeps it
                        up to date automatically. The fields below are then ignored.
                    </p>
                </div>
                <template v-if="!isPackwiz">
                <div class="space-y-2">
                    <Label>Minecraft Version *</Label>
                    <Select :model-value="props.formData.minecraft_version || undefined"
                        :disabled="props.disabled || props.loadingMinecraftVersions"
                        @update:modelValue="(value) => emit('update-field', 'minecraft_version', value?.toString() ?? '')">
                        <SelectTrigger>
                            <SelectValue placeholder="Select version" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem v-for="version in props.minecraftVersions" :key="version" :value="version">
                                {{ version }}
                            </SelectItem>
                        </SelectContent>
                    </Select>
                    <p v-if="props.errors?.minecraft_version" class="text-sm text-destructive">
                        {{ props.errors.minecraft_version }}
                    </p>
                    <p v-else-if="props.loadingMinecraftVersions" class="text-sm text-muted-foreground">
                        Loading versions...
                    </p>
                </div>
                <div class="space-y-2">
                    <Label>Mod Loader *</Label>
                    <Select :model-value="props.formData.loader_name || undefined" :disabled="props.disabled ||
                        props.loadingLoaders ||
                        !props.formData.minecraft_version ||
                        props.availableLoaders.length === 0
                        " @update:modelValue="(value) =>
                            emit(
                                'update-field',
                                'loader_name',
                                (typeof value === 'string' && value.length ? value : LoaderType.VANILLA) as LoaderType,
                            )">
                        <SelectTrigger>
                            <SelectValue placeholder="Select loader" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem v-for="loader in props.availableLoaders" :key="loader" :value="loader">
                                {{ loader }}
                            </SelectItem>
                        </SelectContent>
                    </Select>
                    <p v-if="props.errors?.loader_name" class="text-sm text-destructive">
                        {{ props.errors.loader_name }}
                    </p>
                    <p v-else-if="!props.formData.minecraft_version" class="text-sm text-muted-foreground">
                        Select a Minecraft version first.
                    </p>
                    <p v-else-if="props.availableLoaders.length === 0" class="text-sm text-muted-foreground">
                        No loaders available.
                    </p>
                </div>
                <div class="space-y-2">
                    <Label>Loader Version *</Label>
                    <Select :model-value="props.formData.loader_version || undefined" :disabled="props.disabled ||
                        props.loadingLoaderVersions ||
                        !props.formData.loader_name ||
                        props.loaderVersions.length === 0 ||
                        isVanillaLoader
                        "
                        @update:modelValue="(value) => emit('update-field', 'loader_version', value?.toString() ?? '')">
                        <SelectTrigger>
                            <SelectValue placeholder="Select version" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem v-for="version in props.loaderVersions" :key="version" :value="version">
                                {{ version }}
                            </SelectItem>
                        </SelectContent>
                    </Select>
                    <p v-if="props.errors?.loader_version" class="text-sm text-destructive">
                        {{ props.errors.loader_version }}
                    </p>
                    <p v-else-if="!props.formData.loader_name" class="text-sm text-muted-foreground">
                        Select a loader first.
                    </p>
                    <p v-else-if="!isVanillaLoader && props.loaderVersions.length === 0"
                        class="text-sm text-muted-foreground">
                        No versions available.
                    </p>
                </div>
                </template>
                <div class="space-y-2">
                    <Label>Authentication Type *</Label>
                    <Select :model-value="props.formData.auth_backend.type" :disabled="props.disabled"
                        @update:modelValue="(value) => emit('update-auth-field', 'type', (value as AuthType) ?? AuthType.OFFLINE)">
                        <SelectTrigger>
                            <SelectValue placeholder="Select authentication" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem :value="AuthType.OFFLINE">Offline</SelectItem>
                            <SelectItem :value="AuthType.MOJANG">Mojang</SelectItem>
                            <SelectItem :value="AuthType.TELEGRAM">Telegram</SelectItem>
                            <SelectItem :value="AuthType.ELY_BY">Ely.by</SelectItem>
                        </SelectContent>
                    </Select>
                    <p v-if="props.errors?.auth_type" class="text-sm text-destructive">
                        {{ props.errors.auth_type }}
                    </p>
                </div>
                <div class="space-y-2 sm:col-span-2">
                    <Label :for="`${props.idPrefix}-recommended-xmx`">Recommended Xmx (RAM)</Label>
                    <Input :id="`${props.idPrefix}-recommended-xmx`" :model-value="props.formData.recommended_xmx || ''"
                        :disabled="props.disabled" placeholder="e.g. 4G or 4096M"
                        @update:modelValue="(value) => emit('update-field', 'recommended_xmx', value?.toString() ?? '')" />
                    <p v-if="props.errors?.recommended_xmx" class="text-sm text-destructive">
                        {{ props.errors.recommended_xmx }}
                    </p>
                    <p v-else class="text-sm text-muted-foreground">
                        Optional. Used as the default JVM RAM limit (e.g. <span class="font-mono">4G</span>).
                    </p>
                </div>
            </div>
            <div v-if="props.formData.auth_backend.type === AuthType.TELEGRAM" class="space-y-2">
                <Label :for="`${props.idPrefix}-auth-base-url`">Auth Base URL *</Label>
                <Input :id="`${props.idPrefix}-auth-base-url`" type="url"
                    :model-value="props.formData.auth_backend.auth_base_url || ''" :disabled="props.disabled"
                    placeholder="https://your-telegram-auth-server.com"
                    @update:modelValue="(value) => emit('update-auth-field', 'auth_base_url', value?.toString() ?? '')" />
                <p v-if="props.errors?.auth_base_url" class="text-sm text-destructive">
                    {{ props.errors.auth_base_url }}
                </p>
            </div>
            <div v-if="props.formData.auth_backend.type === AuthType.ELY_BY" class="grid gap-4 sm:grid-cols-2">
                <div class="space-y-2">
                    <Label :for="`${props.idPrefix}-client-id`">Client ID *</Label>
                    <Input :id="`${props.idPrefix}-client-id`"
                        :model-value="props.formData.auth_backend.client_id || ''" :disabled="props.disabled"
                        placeholder="Ely.by client ID"
                        @update:modelValue="(value) => emit('update-auth-field', 'client_id', value?.toString() ?? '')" />
                    <p v-if="props.errors?.client_id" class="text-sm text-destructive">
                        {{ props.errors.client_id }}
                    </p>
                </div>
                <div class="space-y-2">
                    <Label :for="`${props.idPrefix}-client-secret`">Client Secret *</Label>
                    <Input :id="`${props.idPrefix}-client-secret`" type="password"
                        :model-value="props.formData.auth_backend.client_secret || ''" :disabled="props.disabled"
                        placeholder="Ely.by client secret"
                        @update:modelValue="(value) => emit('update-auth-field', 'client_secret', value?.toString() ?? '')" />
                    <p v-if="props.errors?.client_secret" class="text-sm text-destructive">
                        {{ props.errors.client_secret }}
                    </p>
                </div>
            </div>
        </div>

        <div v-if="!isPackwiz" class="space-y-4">
            <div class="flex items-center justify-between">
                <Label>Include Rules</Label>
                <Button type="button" variant="outline" size="sm" class="gap-2" :disabled="props.disabled"
                    @click="emit('add-include-rule')">
                    <Plus class="h-4 w-4" />
                    Add Rule
                </Button>
            </div>

            <div v-if="!props.formData.include?.length" class="text-sm text-muted-foreground italic">
                No include rules defined.
            </div>

            <div v-else class="space-y-3">
                <Card v-for="(rule, index) in props.formData.include" :key="index">
                    <CardContent class="p-4 grid gap-4">
                        <div class="grid gap-2">
                            <Label :for="`${props.idPrefix}-rule-${index}-path`">Path</Label>
                            <Input :id="`${props.idPrefix}-rule-${index}-path`" :model-value="rule.path"
                                :disabled="props.disabled" placeholder="e.g. config/my-mod.cfg"
                                @update:modelValue="(value) => emit('update-include-rule', index, 'path', value?.toString() ?? '')" />
                        </div>
                        <div class="flex flex-wrap gap-6">
                            <label class="flex items-center gap-2 text-sm cursor-pointer">
                                <input type="checkbox" :checked="rule.overwrite" :disabled="props.disabled"
                                    class="h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary disabled:opacity-50"
                                    @change="(e) => emit('update-include-rule', index, 'overwrite', (e.target as HTMLInputElement).checked)" />
                                Overwrite
                            </label>
                            <label class="flex items-center gap-2 text-sm cursor-pointer">
                                <input type="checkbox" :checked="rule.recursive" :disabled="props.disabled"
                                    class="h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary disabled:opacity-50"
                                    @change="(e) => emit('update-include-rule', index, 'recursive', (e.target as HTMLInputElement).checked)" />
                                Recursive
                            </label>
                            <label class="flex items-center gap-2 text-sm cursor-pointer">
                                <input type="checkbox" :checked="rule.delete_extra" :disabled="props.disabled"
                                    class="h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary disabled:opacity-50"
                                    @change="(e) => emit('update-include-rule', index, 'delete_extra', (e.target as HTMLInputElement).checked)" />
                                Delete Extra
                            </label>
                        </div>
                        <div class="flex justify-end">
                            <Button type="button" variant="ghost" size="sm"
                                class="text-destructive hover:text-destructive" :disabled="props.disabled"
                                @click="emit('remove-include-rule', index)">
                                <Trash2 class="h-4 w-4 mr-2" />
                                Remove Rule
                            </Button>
                        </div>
                    </CardContent>
                </Card>
            </div>
        </div>
    </div>
</template>
