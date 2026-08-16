package store

import "github.com/test/features/models"

type Reader interface {
	Get(id string) *models.UserPayload
	Close() error
}

type MemoryStore struct {
	items map[string]*models.UserPayload
}

func (m *MemoryStore) Get(id string) *models.UserPayload {
	return m.items[id]
}

func (m *MemoryStore) Close() error {
	return nil
}
