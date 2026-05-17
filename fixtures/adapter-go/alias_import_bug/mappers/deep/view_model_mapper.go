package deep

import (
    models "github.com/test/app/models"
)

type ViewModelMapper struct{}

func (m *ViewModelMapper) Map(data *models.UserPayload) map[string]interface{} {
    return map[string]interface{}{
        "id": data.UserID,
        "ts": data.Timestamp,
        "st": data.Status,
    }
}
