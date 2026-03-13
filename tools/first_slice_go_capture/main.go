// Historical — first-slice Go truth capture tool.
// The first slice is complete and parity-backed (Phase 1B.6 closure).
// This tool is retained for reference and regression verification.
// Broader parity is now tracked by the three domain ledgers under docs/parity/.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/url"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"

	"github.com/cloudflare/cloudflared/config"
	cfcredentials "github.com/cloudflare/cloudflared/credentials"
	"github.com/cloudflare/cloudflared/ingress"
	"github.com/rs/zerolog"
	"github.com/urfave/cli/v2"
	yaml "gopkg.in/yaml.v3"
	"golang.org/x/net/idna"
)

const schemaVersion = 1
const noIngressRulesFlagsMessage = "No ingress rules were defined in provided config (if any) nor from the provided flags, cloudflared will return 503 for all incoming HTTP requests"

type emissionPlan struct {
	RepoRoot    string        `json:"repo_root"`
	FixtureRoot string        `json:"fixture_root"`
	OutputDir   string        `json:"output_dir"`
	Fixtures    []fixtureSpec `json:"fixtures"`
}

type fixtureSpec struct {
	FixtureID        string         `json:"fixture_id"`
	Category         string         `json:"category"`
	Comparison       string         `json:"comparison"`
	Input            string         `json:"input"`
	SourceRefs       []string       `json:"source_refs"`
	DiscoveryCase    *discoveryCase `json:"discovery_case,omitempty"`
	OriginCertSource *string        `json:"origin_cert_source,omitempty"`
	OrderingCase     *orderingCase  `json:"ordering_case,omitempty"`
	FlagIngressCase   *flagIngressCase `json:"flag_ingress_case,omitempty"`
}

type discoveryCase struct {
	ExplicitConfig bool     `json:"explicit_config"`
	Present        []string `json:"present"`
}

type orderingCase struct {
	Input string `json:"input"`
}

type flagIngressCase struct {
	Flags []string `json:"flags"`
}

type artifactEnvelope struct {
	SchemaVersion uint32          `json:"schema_version"`
	FixtureID     string          `json:"fixture_id"`
	Producer      string          `json:"producer"`
	ReportKind    string          `json:"report_kind"`
	Comparison    string          `json:"comparison"`
	SourceRefs    []string        `json:"source_refs"`
	Payload       json.RawMessage `json:"payload"`
}

type discoveryReportPayload struct {
	Action       string   `json:"action"`
	SourceKind   string   `json:"source_kind"`
	ResolvedPath string   `json:"resolved_path"`
	CreatedPaths []string `json:"created_paths"`
	WrittenConfig *string `json:"written_config"`
}

type errorReportPayload struct {
	Category string `json:"category"`
	Message  string `json:"message"`
}

type credentialReportPayload struct {
	Kind          string  `json:"kind"`
	SourcePath    string  `json:"source_path"`
	ZoneID        string  `json:"zone_id"`
	AccountID     string  `json:"account_id"`
	APIToken      string  `json:"api_token"`
	Endpoint      *string `json:"endpoint"`
	IsFedEndpoint bool    `json:"is_fed_endpoint"`
}

type ingressReportPayload struct {
	SourceKind        string               `json:"source_kind"`
	RuleCount         int                  `json:"rule_count"`
	CatchAllRuleIndex int                  `json:"catch_all_rule_index"`
	Defaults         originRequestPayload `json:"defaults"`
	Rules            []ingressRulePayload `json:"rules"`
}

type normalizedConfigPayload struct {
	SourceKind    string                   `json:"source_kind"`
	SourcePath    string                   `json:"source_path"`
	Tunnel        *tunnelReferencePayload  `json:"tunnel"`
	Credentials   credentialSurfacePayload `json:"credentials"`
	Ingress       []ingressRulePayload     `json:"ingress"`
	OriginRequest originRequestPayload     `json:"origin_request"`
	WarpRouting   warpRoutingPayload       `json:"warp_routing"`
	LogDirectory  *string                  `json:"log_directory"`
	Warnings      []warningPayload         `json:"warnings"`
}

type tunnelReferencePayload struct {
	Raw  string  `json:"raw"`
	UUID *string `json:"uuid"`
}

type credentialSurfacePayload struct {
	CredentialsFile *string                   `json:"credentials_file"`
	OriginCert      *originCertLocatorPayload `json:"origin_cert"`
	Tunnel          *tunnelReferencePayload   `json:"tunnel"`
}

type originCertLocatorPayload struct {
	Kind string `json:"kind"`
	Path string `json:"path"`
}

type ingressRulePayload struct {
	Hostname         *string              `json:"hostname"`
	PunycodeHostname *string              `json:"punycode_hostname"`
	Path             *string              `json:"path"`
	Service          ingressServicePayload `json:"service"`
	OriginRequest    originRequestPayload `json:"origin_request"`
}

type ingressServicePayload struct {
	Kind       string  `json:"kind"`
	URI        *string `json:"uri,omitempty"`
	Path       *string `json:"path,omitempty"`
	Name       *string `json:"name,omitempty"`
	StatusCode *int    `json:"status_code,omitempty"`
}

type warningPayload struct {
	Kind string   `json:"kind"`
	Keys []string `json:"keys"`
}

type warpRoutingPayload struct {
	ConnectTimeout *string `json:"connectTimeout"`
	MaxActiveFlows *uint64 `json:"maxActiveFlows"`
	TCPKeepAlive   *string `json:"tcpKeepAlive"`
}

type originRequestPayload struct {
	ConnectTimeout         *string             `json:"connectTimeout"`
	TLSTimeout             *string             `json:"tlsTimeout"`
	TCPKeepAlive           *string             `json:"tcpKeepAlive"`
	NoHappyEyeballs        *bool               `json:"noHappyEyeballs"`
	KeepAliveConnections   *int                `json:"keepAliveConnections"`
	KeepAliveTimeout       *string             `json:"keepAliveTimeout"`
	HTTPHostHeader         *string             `json:"httpHostHeader"`
	OriginServerName       *string             `json:"originServerName"`
	MatchSNIToHost         *bool               `json:"matchSNItoHost"`
	CAPool                 *string             `json:"caPool"`
	NoTLSVerify            *bool               `json:"noTLSVerify"`
	DisableChunkedEncoding *bool               `json:"disableChunkedEncoding"`
	BastionMode            *bool               `json:"bastionMode"`
	ProxyAddress           *string             `json:"proxyAddress"`
	ProxyPort              *uint               `json:"proxyPort"`
	ProxyType              *string             `json:"proxyType"`
	IPRules                []ingressIPRulePayload `json:"ipRules"`
	HTTP2Origin            *bool               `json:"http2Origin"`
	Access                 *accessPayload      `json:"access"`
}

type ingressIPRulePayload struct {
	Prefix *string `json:"prefix"`
	Ports  []int   `json:"ports"`
	Allow  bool    `json:"allow"`
}

type accessPayload struct {
	Required    bool     `json:"required"`
	TeamName    string   `json:"teamName"`
	AudTag      []string `json:"audTag"`
	Environment *string  `json:"environment"`
}

type yamlConfigFile struct {
	config.Configuration `yaml:",inline"`
	Settings             map[string]any `yaml:",inline"`
}

type yamlStrictConfigFile struct {
	config.Configuration `yaml:",inline"`
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run() error {
	plan, err := readPlan(os.Stdin)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(plan.OutputDir, 0o755); err != nil {
		return err
	}

	for _, fixture := range plan.Fixtures {
		envelope, err := emitFixture(plan, fixture)
		if err != nil {
			return fmt.Errorf("emit %s: %w", fixture.FixtureID, err)
		}
		encoded, err := json.MarshalIndent(envelope, "", "  ")
		if err != nil {
			return err
		}
		outputPath := filepath.Join(plan.OutputDir, fixture.FixtureID+".json")
		if err := os.WriteFile(outputPath, append(encoded, '\n'), 0o644); err != nil {
			return err
		}
	}
	return nil
}

func readPlan(reader io.Reader) (emissionPlan, error) {
	var plan emissionPlan
	if err := json.NewDecoder(reader).Decode(&plan); err != nil {
		return emissionPlan{}, err
	}
	return plan, nil
}

func emitFixture(plan emissionPlan, fixture fixtureSpec) (artifactEnvelope, error) {
	switch fixture.Category {
	case "config-discovery":
		return emitDiscoveryFixture(fixture)
	case "yaml-config":
		return emitNormalizedConfigFixture(plan, fixture, fixture.Input, false)
	case "invalid-input":
		return emitNormalizedConfigFixture(plan, fixture, fixture.Input, false)
	case "ordering-defaulting":
		input := fixture.Input
		if fixture.OrderingCase != nil {
			input = fixture.OrderingCase.Input
		}
		return emitNormalizedConfigFixture(plan, fixture, input, true)
	case "credentials-origin-cert":
		return emitOriginCertFixture(plan, fixture)
	case "ingress-normalization":
		return emitFlagIngressFixture(fixture)
	default:
		return artifactEnvelope{}, fmt.Errorf("unsupported fixture category: %s", fixture.Category)
	}
}

func emitDiscoveryFixture(fixture fixtureSpec) (artifactEnvelope, error) {
	if fixture.DiscoveryCase == nil {
		return artifactEnvelope{}, fmt.Errorf("fixture %s missing discovery case", fixture.FixtureID)
	}
	sandboxRoot, err := os.MkdirTemp("", "cloudflared-go-discovery-")
	if err != nil {
		return artifactEnvelope{}, err
	}
	defer os.RemoveAll(sandboxRoot)

	for _, logicalPath := range fixture.DiscoveryCase.Present {
		actualPath := filepath.Join(sandboxRoot, logicalPath)
		if err := os.MkdirAll(filepath.Dir(actualPath), 0o755); err != nil {
			return artifactEnvelope{}, err
		}
		if err := os.WriteFile(actualPath, []byte("logDirectory: /var/log/cloudflared\n"), 0o644); err != nil {
			return artifactEnvelope{}, err
		}
	}

	payload, err := simulateDiscovery(sandboxRoot, *fixture.DiscoveryCase)
	if err != nil {
		return artifactEnvelope{}, err
	}
	return newEnvelope(fixture, "go-truth", "discovery-report.v1", payload)
}

func simulateDiscovery(sandboxRoot string, dc discoveryCase) (discoveryReportPayload, error) {
	configNames := []string{"config.yml", "config.yaml"}
	searchDirs := []string{
		"home/.cloudflared",
		"home/.cloudflare-warp",
		"home/cloudflare-warp",
		"etc/cloudflared",
		"usr/local/etc/cloudflared",
	}
	for _, dir := range searchDirs {
		for _, name := range configNames {
			candidate := filepath.Join(sandboxRoot, dir, name)
			if fileExists(candidate) {
				return discoveryReportPayload{
					Action:       "use-existing",
					SourceKind:   pickDiscoverySourceKind(dc, dir),
					ResolvedPath: displaySandboxPath(sandboxRoot, candidate),
					CreatedPaths: []string{},
					WrittenConfig: nil,
				}, nil
			}
		}
	}

	configPath := filepath.Join(sandboxRoot, "usr/local/etc/cloudflared/config.yml")
	logDir := filepath.Join(sandboxRoot, "var/log/cloudflared")
	if err := os.MkdirAll(filepath.Dir(configPath), 0o755); err != nil {
		return discoveryReportPayload{}, err
	}
	if err := os.MkdirAll(logDir, 0o755); err != nil {
		return discoveryReportPayload{}, err
	}
	writtenConfig := fmt.Sprintf("logDirectory: %s\n", logDir)
	if err := os.WriteFile(configPath, []byte(writtenConfig), 0o644); err != nil {
		return discoveryReportPayload{}, err
	}
	return discoveryReportPayload{
		Action:       "create-default-config",
		SourceKind:   "auto-created-path",
		ResolvedPath: "/usr/local/etc/cloudflared/config.yml",
		CreatedPaths: []string{
			"/usr/local/etc/cloudflared",
			"/usr/local/etc/cloudflared/config.yml",
			"/var/log/cloudflared",
		},
		WrittenConfig: stringPtr(writtenConfig),
	}, nil
}

func pickDiscoverySourceKind(dc discoveryCase, logicalDir string) string {
	if dc.ExplicitConfig && logicalDir == "home/.cloudflared" {
		return "explicit-path"
	}
	return "discovered-path"
}

func emitNormalizedConfigFixture(plan emissionPlan, fixture fixtureSpec, input string, useConfigAndCLI bool) (artifactEnvelope, error) {
	inputPath := filepath.Join(plan.FixtureRoot, input)
	configuration, warnings, err := loadYAMLConfig(inputPath)
	if err != nil {
		return newErrorEnvelope(fixture, classifyConfigError(err), err.Error())
	}

	var ing ingress.Ingress
	if useConfigAndCLI {
		cliCtx := newFlagContext(nil)
		logger := zerolog.Nop()
		ing, err = ingress.ParseIngressFromConfigAndCLI(configuration, cliCtx, &logger)
	} else {
		ing, err = ingress.ParseIngress(configuration)
	}
	if err != nil {
		return newErrorEnvelope(fixture, classifyConfigError(err), err.Error())
	}

	payload := normalizedConfigPayload{
		SourceKind:    "discovered-path",
		SourcePath:    input,
		Tunnel:        tunnelReference(configuration.TunnelID),
		Credentials:   credentialSurfacePayload{Tunnel: tunnelReference(configuration.TunnelID)},
		Ingress:       canonicalIngressRules(ing.Rules),
		OriginRequest: canonicalOriginRequest(ing.Defaults),
		WarpRouting:   canonicalWarpRouting(configuration.WarpRouting),
		LogDirectory:  nil,
		Warnings:      warnings,
	}
	return newEnvelope(fixture, "go-truth", "normalized-config.v1", payload)
}

func emitOriginCertFixture(plan emissionPlan, fixture fixtureSpec) (artifactEnvelope, error) {
	if fixture.OriginCertSource == nil {
		return artifactEnvelope{}, fmt.Errorf("fixture %s missing origin cert source", fixture.FixtureID)
	}
	inputPath := filepath.Join(plan.RepoRoot, *fixture.OriginCertSource)
	logger := zerolog.Nop()
	user, err := cfcredentials.Read(inputPath, &logger)
	if err != nil {
		return newErrorEnvelope(fixture, classifyCredentialError(err), err.Error())
	}
	payload := credentialReportPayload{
		Kind:          "origin-cert-pem",
		SourcePath:    *fixture.OriginCertSource,
		ZoneID:        user.ZoneID(),
		AccountID:     user.AccountID(),
		APIToken:      user.APIToken(),
		Endpoint:      nilIfEmpty(user.Endpoint()),
		IsFedEndpoint: user.IsFEDEndpoint(),
	}
	return newEnvelope(fixture, "go-truth", "credential-report.v1", payload)
}

func emitFlagIngressFixture(fixture fixtureSpec) (artifactEnvelope, error) {
	if fixture.FlagIngressCase == nil {
		return artifactEnvelope{}, fmt.Errorf("fixture %s missing flag ingress case", fixture.FixtureID)
	}
	if !hasFlagOrigin(fixture.FlagIngressCase.Flags) {
		return newErrorEnvelope(fixture, "no-ingress-rules-flags", noIngressRulesFlagsMessage)
	}
	cliCtx := newFlagContext(fixture.FlagIngressCase.Flags)
	logger := zerolog.Nop()
	ing, err := ingress.ParseIngressFromConfigAndCLI(&config.Configuration{}, cliCtx, &logger)
	if err != nil {
		return newErrorEnvelope(fixture, classifyConfigError(err), err.Error())
	}
	payload := ingressReportPayload{
		SourceKind:        "flag-single-origin",
		RuleCount:         len(ing.Rules),
		CatchAllRuleIndex: len(ing.Rules) - 1,
		Defaults:          canonicalOriginRequest(ing.Defaults),
		Rules:             canonicalIngressRules(ing.Rules),
	}
	return newEnvelope(fixture, "go-truth", "ingress-report.v1", payload)
}

func newEnvelope(fixture fixtureSpec, producer string, reportKind string, payload any) (artifactEnvelope, error) {
	encodedPayload, err := json.Marshal(payload)
	if err != nil {
		return artifactEnvelope{}, err
	}
	return artifactEnvelope{
		SchemaVersion: schemaVersion,
		FixtureID:     fixture.FixtureID,
		Producer:      producer,
		ReportKind:    reportKind,
		Comparison:    fixture.Comparison,
		SourceRefs:    fixture.SourceRefs,
		Payload:       encodedPayload,
	}, nil
}

func newErrorEnvelope(fixture fixtureSpec, category string, message string) (artifactEnvelope, error) {
	return newEnvelope(fixture, "go-truth", "error-report.v1", errorReportPayload{
		Category: category,
		Message:  message,
	})
}

func canonicalIngressRules(rules []ingress.Rule) []ingressRulePayload {
	canonical := make([]ingressRulePayload, 0, len(rules))
	for _, rule := range rules {
		canonical = append(canonical, ingressRulePayload{
			Hostname:         nilIfEmpty(rule.Hostname),
			PunycodeHostname: punycodeHostname(rule.Hostname),
			Path:             regexString(rule.Path),
			Service:          canonicalService(rule.Service.String()),
			OriginRequest:    canonicalOriginRequest(rule.Config),
		})
	}
	return canonical
}

func canonicalService(value string) ingressServicePayload {
	if strings.HasPrefix(value, "unix+tls:") {
		path := strings.TrimPrefix(value, "unix+tls:")
		return ingressServicePayload{Kind: "unix-socket-tls", Path: &path}
	}
	if strings.HasPrefix(value, "unix:") {
		path := strings.TrimPrefix(value, "unix:")
		return ingressServicePayload{Kind: "unix-socket", Path: &path}
	}
	if strings.HasPrefix(value, "http_status:") {
		status, _ := strconvAtoi(strings.TrimPrefix(value, "http_status:"))
		return ingressServicePayload{Kind: "http-status", StatusCode: &status}
	}
	if value == ingress.HelloWorldService {
		return ingressServicePayload{Kind: "hello-world"}
	}
	if value == ingress.ServiceBastion {
		return ingressServicePayload{Kind: "bastion"}
	}
	if value == ingress.ServiceSocksProxy {
		return ingressServicePayload{Kind: "socks-proxy"}
	}
	if parsed, err := url.Parse(value); err == nil && parsed.Scheme != "" && parsed.Hostname() != "" {
		rendered := displayOriginURL(parsed)
		if parsed.Scheme == "http" || parsed.Scheme == "https" || parsed.Scheme == "ws" || parsed.Scheme == "wss" {
			return ingressServicePayload{Kind: "http", URI: &rendered}
		}
		return ingressServicePayload{Kind: "tcp-over-websocket", URI: &rendered}
	}
	return ingressServicePayload{Kind: "named-token", Name: &value}
}

func canonicalOriginRequest(cfg ingress.OriginRequestConfig) originRequestPayload {
	ipRules := make([]ingressIPRulePayload, 0, len(cfg.IPRules))
	for _, rule := range cfg.IPRules {
		rulePorts := rule.Ports()
		ports := make([]int, len(rulePorts))
		copy(ports, rulePorts)
		prefix := rule.StringCIDR()
		ipRules = append(ipRules, ingressIPRulePayload{
			Prefix: &prefix,
			Ports:  ports,
			Allow:  rule.RulePolicy(),
		})
	}
	return originRequestPayload{
		ConnectTimeout:         durationString(cfg.ConnectTimeout),
		TLSTimeout:             durationString(cfg.TLSTimeout),
		TCPKeepAlive:           durationString(cfg.TCPKeepAlive),
		NoHappyEyeballs:        boolPtr(cfg.NoHappyEyeballs),
		KeepAliveConnections:   intPtr(cfg.KeepAliveConnections),
		KeepAliveTimeout:       durationString(cfg.KeepAliveTimeout),
		HTTPHostHeader:         nilIfEmpty(cfg.HTTPHostHeader),
		OriginServerName:       nilIfEmpty(cfg.OriginServerName),
		MatchSNIToHost:         boolPtr(cfg.MatchSNIToHost),
		CAPool:                 nilIfEmpty(cfg.CAPool),
		NoTLSVerify:            boolPtr(cfg.NoTLSVerify),
		DisableChunkedEncoding: boolPtr(cfg.DisableChunkedEncoding),
		BastionMode:            boolPtr(cfg.BastionMode),
		ProxyAddress:           nilIfEmpty(cfg.ProxyAddress),
		ProxyPort:              uintPtr(cfg.ProxyPort),
		ProxyType:              nilIfEmpty(cfg.ProxyType),
		IPRules:                ipRules,
		HTTP2Origin:            boolPtr(cfg.Http2Origin),
		Access:                 canonicalAccess(cfg.Access),
	}
}

func canonicalWarpRouting(cfg config.WarpRoutingConfig) warpRoutingPayload {
	return warpRoutingPayload{
		ConnectTimeout: customDurationString(cfg.ConnectTimeout),
		MaxActiveFlows: cfg.MaxActiveFlows,
		TCPKeepAlive:   customDurationString(cfg.TCPKeepAlive),
	}
}

func canonicalAccess(cfg config.AccessConfig) *accessPayload {
	if !cfg.Required && cfg.TeamName == "" && len(cfg.AudTag) == 0 && cfg.Environment == "" {
		return nil
	}
	return &accessPayload{
		Required:    cfg.Required,
		TeamName:    cfg.TeamName,
		AudTag:      append([]string(nil), cfg.AudTag...),
		Environment: nilIfEmpty(cfg.Environment),
	}
}

func tunnelReference(raw string) *tunnelReferencePayload {
	if raw == "" {
		return nil
	}
	return &tunnelReferencePayload{Raw: raw, UUID: nil}
}

func loadYAMLConfig(path string) (*config.Configuration, []warningPayload, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, nil, err
	}
	defer file.Close()

	var settings yamlConfigFile
	if err := yaml.NewDecoder(file).Decode(&settings); err != nil {
		return nil, nil, err
	}

	strictFile, err := os.Open(path)
	if err != nil {
		return nil, nil, err
	}
	defer strictFile.Close()
	decoder := yaml.NewDecoder(strictFile)
	decoder.KnownFields(true)
	var strictSettings yamlStrictConfigFile
	var warnings []warningPayload
	if err := decoder.Decode(&strictSettings); err != nil {
		warnings = parseWarningPayload(err.Error())
	}

	return &settings.Configuration, warnings, nil
}

func parseWarningPayload(message string) []warningPayload {
	matches := regexp.MustCompile(`field ([^ ]+) not found`).FindAllStringSubmatch(message, -1)
	if len(matches) == 0 {
		return nil
	}
	seen := map[string]struct{}{}
	keys := make([]string, 0, len(matches))
	for _, match := range matches {
		key := match[1]
		if _, ok := seen[key]; ok {
			continue
		}
		seen[key] = struct{}{}
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return []warningPayload{{Kind: "unknown-top-level-keys", Keys: keys}}
}

func classifyConfigError(err error) string {
	message := err.Error()
	switch {
	case strings.Contains(message, "The last ingress rule must match all URLs"):
		return "ingress-last-rule-not-catch-all"
	case strings.Contains(message, "Hostname patterns can have at most one wildcard"):
		return "ingress-bad-wildcard"
	case strings.Contains(message, "Hostname cannot contain a port"):
		return "ingress-hostname-contains-port"
	case strings.Contains(message, "No ingress rules were defined"):
		return "no-ingress-rules-flags"
	default:
		return "invariant-violation"
	}
}

func classifyCredentialError(err error) string {
	message := err.Error()
	switch {
	case strings.Contains(message, "cannot decode empty certificate"):
		return "origin-cert-empty"
	case strings.Contains(message, "unknown block"):
		return "origin-cert-unknown-block"
	case strings.Contains(message, "found multiple tokens"):
		return "origin-cert-multiple-tokens"
	case strings.Contains(message, "missing token in the certificate"):
		return "origin-cert-missing-token"
	case strings.Contains(message, "Origin certificate needs to be refreshed"):
		return "origin-cert-needs-refresh"
	default:
		return "io"
	}
}

func newFlagContext(flags []string) *cli.Context {
	flagSet := flag.NewFlagSet("first-slice-capture", flag.ContinueOnError)
	flagSet.Bool(ingress.HelloWorldFlag, false, "")
	flagSet.Bool(config.BastionFlag, false, "")
	flagSet.String("url", "", "")
	flagSet.String("unix-socket", "", "")
	cliCtx := cli.NewContext(cli.NewApp(), flagSet, nil)
	for _, raw := range flags {
		name, value := splitFlagArg(raw)
		_ = cliCtx.Set(name, value)
	}
	return cliCtx
}

func hasFlagOrigin(flags []string) bool {
	for _, raw := range flags {
		name, value := splitFlagArg(raw)
		switch name {
		case ingress.HelloWorldFlag, config.BastionFlag:
			if value == "true" || value == "" {
				return true
			}
		case "url", "unix-socket":
			if value != "" {
				return true
			}
		}
	}
	return false
}

func splitFlagArg(raw string) (string, string) {
	trimmed := strings.TrimPrefix(raw, "--")
	if name, value, ok := strings.Cut(trimmed, "="); ok {
		return name, value
	}
	return trimmed, "true"
}

func displayOriginURL(parsed *url.URL) string {
	rendered := parsed.String()
	if parsed.Path == "/" && parsed.RawQuery == "" && parsed.Fragment == "" {
		return strings.TrimSuffix(rendered, "/")
	}
	return rendered
}

func punycodeHostname(hostname string) *string {
	if hostname == "" || hostname == "*" || strings.Contains(hostname, "*") {
		return nil
	}
	punycode, err := idna.Lookup.ToASCII(hostname)
	if err != nil || punycode == hostname {
		return nil
	}
	return &punycode
}

func regexString(pattern *ingress.Regexp) *string {
	if pattern == nil || pattern.Regexp == nil {
		return nil
	}
	s := pattern.String()
	return &s
}

func durationString(value config.CustomDuration) *string {
	text := value.Duration.String()
	if text == "0s" {
		return nil
	}
	return &text
}

func customDurationString(value *config.CustomDuration) *string {
	if value == nil {
		return nil
	}
	text := value.Duration.String()
	return &text
}

func boolPtr(value bool) *bool {
	return &value
}

func intPtr(value int) *int {
	return &value
}

func uintPtr(value uint) *uint {
	return &value
}

func nilIfEmpty(value string) *string {
	if value == "" {
		return nil
	}
	return &value
}

func stringPtr(value string) *string {
	return &value
}

func displaySandboxPath(sandboxRoot string, path string) string {
	relative, err := filepath.Rel(sandboxRoot, path)
	if err != nil || strings.HasPrefix(relative, "..") {
		return path
	}
	return "/" + filepath.ToSlash(relative)
}

func fileExists(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return !info.IsDir()
}

func strconvAtoi(value string) (int, error) {
	parsed, err := strconv.ParseInt(value, 10, 64)
	return int(parsed), err
}
