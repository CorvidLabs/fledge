package cmd

import "testing"

func TestVersion(t *testing.T) {
	if version == "" {
		t.Error("version should not be empty")
	}
}
