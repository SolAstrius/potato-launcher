package api

import "github.com/Petr1Furious/potato-launcher/backend/internal/models"

type TokenRequest struct {
	Token string `json:"token" doc:"Admin secret token"`
}

type TokenResponse struct {
	AccessToken string `json:"access_token"`
	TokenType   string `json:"token_type"`
}

type APISettings struct {
	ReplaceDownloadURLs bool `json:"replace_download_urls" doc:"Whether to replace download URLs in the client"`
}

type APIInstance struct {
	Name             string               `json:"name" example:"survival-1.21"`
	PackwizURL       string               `json:"packwiz_url,omitempty" example:"https://mc.sol.moe/pack/pack.toml" doc:"If set, this is a packwiz pack the client generates locally; minecraft_version/loader/include are ignored"`
	MinecraftVersion string               `json:"minecraft_version,omitempty" example:"1.21.1"`
	LoaderName       models.LoaderType    `json:"loader_name,omitempty" example:"fabric"`
	LoaderVersion    string               `json:"loader_version,omitempty" example:"0.15.11"`
	Include          []models.IncludeRule `json:"include,omitempty"`
	AuthBackend      *models.AuthBackend  `json:"auth_backend,omitempty"`
	RecommendedXmx   string               `json:"recommended_xmx,omitempty" example:"4G"`
}

type APISpec struct {
	ReplaceDownloadURLs bool          `json:"replace_download_urls"`
	Instances           []APIInstance `json:"instances"`
}

type BuildStatusResponse struct {
	Status models.BuildStatus `json:"status"`
}
