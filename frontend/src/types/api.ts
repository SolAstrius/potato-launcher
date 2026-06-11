export enum LoaderType {
  VANILLA = 'vanilla',
  FORGE = 'forge',
  FABRIC = 'fabric',
  NEOFORGE = 'neoforge',
}

export enum AuthType {
  OFFLINE = 'offline',
  MOJANG = 'mojang',
  TELEGRAM = 'telegram',
  ELY_BY = 'ely.by',
}

export interface AuthBackend {
  type: AuthType;
  auth_base_url?: string;
  client_id?: string;
  client_secret?: string;
}

export interface IncludeRule {
  path: string;
  overwrite?: boolean;
  recursive?: boolean;
  delete_extra?: boolean;
}

export interface InstanceResponse {
  name: string;
  /** If set, a packwiz pack the client generates locally; version/loader/include are unused. */
  packwiz_url?: string;
  minecraft_version: string;
  loader_name: LoaderType;
  loader_version?: string;
  auth_backend: AuthBackend;
  include?: IncludeRule[];
  recommended_xmx?: string;
}

export interface InstanceBase {
  name: string;
  /** If set, a packwiz pack the client generates locally; version/loader/include are unused. */
  packwiz_url?: string;
  minecraft_version: string;
  loader_name: LoaderType;
  loader_version?: string;
  auth_backend: AuthBackend;
  include?: IncludeRule[];
  recommended_xmx?: string;
}

export interface Settings {
  replace_download_urls: boolean;
}
