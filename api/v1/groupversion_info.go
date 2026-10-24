package v1

import (
	"k8s.io/apimachinery/pkg/runtime/schema"
)

// GroupVersion is the API group and version for Factory resources.
const GroupName = "factory.earendil.works"
const Version = "v1"

// GroupVersionKind is the GVK for Factory resources.
var GroupVersionKind = schema.GroupVersionKind{
	Group:   GroupName,
	Version: Version,
	Kind:    "Factory",
}

// SchemeGroupVersion is the GroupVersion for this API.
var SchemeGroupVersion = schema.GroupVersion{
	Group:   GroupName,
	Version: Version,
}

// Kind takes an unqualified kind and returns a GroupVersion.
func Kind(kind string) schema.GroupKind {
	return SchemeGroupVersion.WithKind(kind).GroupKind()
}

// Resource takes an unqualified resource and returns a GroupResource.
func Resource(resource string) schema.GroupResource {
	return SchemeGroupVersion.WithResource(resource).GroupResource()
}
