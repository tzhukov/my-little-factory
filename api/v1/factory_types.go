package v1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// +genclient
// +k8s:deepcopy-gen:interfaces=k8s.io/apimachinery/pkg/runtime/interfaces

// Factory is the CRD for a software factory job.
type Factory struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   FactorySpec   `json:"spec,omitempty"`
	Status FactoryStatus `json:"status,omitempty"`
}

// FactorySpec is the desired state of a Factory.
type FactorySpec struct {
	// Model references the model configuration.
	Model ModelConfig `json:"model"`

	// Repo is the source repository.
	Repo RepoConfig `json:"repo"`

	// Task is the user's request.
	Task string `json:"task"`

	// Stages lists the stages to run in order.
	// Controller-managed but modular for future user-configurable pivot.
	Stages []StageName `json:"stages"`

	// Resources is the default resource allocation for worker pods.
	Resources WorkerResources `json:"resources,omitempty"`
}

// ModelConfig references model configuration via k8s Secret.
type ModelConfig struct {
	// APIKeySecretRef references the secret containing the API key.
	APIKeySecretRef SecretKeyReference `json:"apiKeySecretRef"`

	// EndpointSecretRef references the secret containing the model endpoint URL.
	EndpointSecretRef SecretKeyReference `json:"endpointSecretRef"`

	// Model is the model name.
	Model string `json:"model"`
}

// SecretKeyReference references a key in a k8s Secret.
type SecretKeyReference struct {
	// Name of the secret.
	Name string `json:"name"`

	// Key in the secret.
	Key string `json:"key"`
}

// RepoConfig describes the source repository.
type RepoConfig struct {
	// URL of the repository (git@github.com:... or file:///...).
	URL string `json:"url"`

	// Branch to use (default: main).
	Branch string `json:"branch,omitempty"`

	// Ref overrides branch (commit SHA, tag, PR ref).
	Ref string `json:"ref,omitempty"`
}

// StageName is a named stage in the pipeline.
type StageName string

const (
	StagePlan  StageName = "plan"
	StageCode  StageName = "code"
	StageTest  StageName = "test"
	StageReview StageName = "review"
)

// WorkerResources describes default resource allocation for worker pods.
type WorkerResources struct {
	CPU    string `json:"cpu,omitempty"`
	Memory string `json:"memory,omitempty"`
}

// FactoryStatus is the observed state of a Factory.
type FactoryStatus struct {
	// Phase is the overall lifecycle phase.
	Phase FactoryPhase `json:"phase"`

	// Stages tracks the state of each stage.
	Stages []StageStatus `json:"stages"`

	// Result is the final output of the pipeline.
	Result string `json:"result,omitempty"`
}

// FactoryPhase indicates the lifecycle phase of a Factory.
type FactoryPhase string

const (
	FactoryPending  FactoryPhase = "Pending"
	FactoryRunning  FactoryPhase = "Running"
	FactoryComplete FactoryPhase = "Complete"
	FactoryFailed   FactoryPhase = "Failed"
)

// StageStatus tracks the state of a single stage.
type StageStatus struct {
	Name        StageName  `json:"name"`
	Phase       StagePhase `json:"phase"`
	StartedAt   *metav1.Time `json:"startedAt,omitempty"`
	CompletedAt *metav1.Time `json:"completedAt,omitempty"`
	// Result is the structured output of this stage.
	Result StageResult `json:"result,omitempty"`
	// Error captured if this stage failed.
	Error string `json:"error,omitempty"`
}

// StagePhase indicates the lifecycle phase of a stage.
type StagePhase string

const (
	StagePending  StagePhase = "Pending"
	StageRunning  StagePhase = "Running"
	StageComplete StagePhase = "Complete"
	StageFailed   StagePhase = "Failed"
)

// StageResult is the structured output of a stage.
type StageResult struct {
	// Text is plain text output (plan, review comments, etc.).
	Text string `json:"text,omitempty"`

	// Diff is the code diff output.
	Diff string `json:"diff,omitempty"`

	// TestOutput is the test runner output.
	TestOutput string `json:"testOutput,omitempty"`

	// Artifacts are files passed between stages (base64 encoded).
	Artifacts map[string]string `json:"artifacts,omitempty"`
}

// +k8s:deepcopy-gen:interfaces=k8s.io/apimachinery/pkg/runtime.Object

// FactoryList is a list of Factory resources.
type FactoryList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`

	Items []Factory `json:"items"`
}
