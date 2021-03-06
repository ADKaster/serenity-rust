/*
 * Copyright (c) 2022, Valtteri Koskivuori <vkoskiv@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

#include "LocalStorageModel.h"

namespace Browser {

void LocalStorageModel::set_items(OrderedHashMap<String, String> map)
{
    begin_insert_rows({}, m_local_storage_entries.size(), m_local_storage_entries.size());
    m_local_storage_entries = map;
    end_insert_rows();

    did_update(DontInvalidateIndices);
}

void LocalStorageModel::clear_items()
{
    begin_insert_rows({}, m_local_storage_entries.size(), m_local_storage_entries.size());
    m_local_storage_entries.clear();
    end_insert_rows();

    did_update(DontInvalidateIndices);
}

String LocalStorageModel::column_name(int column) const
{
    switch (column) {
    case Column::Key:
        return "Key";
    case Column::Value:
        return "Value";
    case Column::__Count:
        return {};
    }

    return {};
}

GUI::ModelIndex LocalStorageModel::index(int row, int column, GUI::ModelIndex const&) const
{
    if (static_cast<size_t>(row) < m_local_storage_entries.size())
        return create_index(row, column, NULL);
    return {};
}

GUI::Variant LocalStorageModel::data(GUI::ModelIndex const& index, GUI::ModelRole role) const
{
    if (role != GUI::ModelRole::Display)
        return {};

    auto const& keys = m_local_storage_entries.keys();
    auto const& local_storage_key = keys[index.row()];
    auto const& local_storage_value = m_local_storage_entries.get(local_storage_key).value_or({});

    switch (index.column()) {
    case Column::Key:
        return local_storage_key;
    case Column::Value:
        return local_storage_value;
    }

    VERIFY_NOT_REACHED();
}

}
