// GTRM_* shim: OpenDRIVE lane-section / lane structure (ids, types, links,
// boundaries, offsets, friction), which the stock esminiRMLib C API only
// exposes as a flat per-(road, s) lane list.
//
// See gt_common.hpp for the shim conventions. We never modify external/esmini.

#include "gt_common.hpp"

using namespace roadmanager;

// One <laneSection> of a road. `s` is its start along the road; `length` its
// extent; `n_lanes` the lane count (including the center/reference lane).
struct GTRM_LaneSection
{
    unsigned int road_id;
    double       s;
    double       length;
    int          n_lanes;
};

// One lane within a lane section. `lane_type` is the roadmanager::Lane::LaneType
// bitmask; `global_id` the OSI global id. `has_pred`/`has_succ` flag whether a
// predecessor/successor lane link exists, with the connected lane id in
// `pred_lane_id`/`succ_lane_id` when so.
struct GTRM_Lane
{
    unsigned int road_id;
    unsigned int section_idx;
    int          lane_id;
    int          lane_type;
    unsigned int global_id;
    int          is_road_edge;
    int          has_pred;
    int          pred_lane_id;
    int          has_succ;
    int          succ_lane_id;
};

namespace
{
    // Resolve (road_id, section_idx) -> LaneSection*, or null.
    LaneSection* section_of(unsigned int road_id, unsigned int section_idx)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return nullptr;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr || section_idx >= road->GetNumberOfLaneSections())
        {
            return nullptr;
        }
        return road->GetLaneSectionByIdx(section_idx);
    }
}  // namespace

extern "C"
{
    // Number of lane sections on `road_id`; -1 on error.
    int GTRM_GetNumberOfLaneSections(unsigned int road_id)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr)
        {
            return -1;
        }
        return static_cast<int>(road->GetNumberOfLaneSections());
    }

    // Fill `out` with lane section `section_idx` of `road_id`. 0 / -1.
    int GTRM_GetLaneSection(unsigned int road_id, unsigned int section_idx, GTRM_LaneSection* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        LaneSection* ls = section_of(road_id, section_idx);
        if (ls == nullptr)
        {
            return -1;
        }
        out->road_id = road_id;
        out->s       = ls->GetS();
        out->length  = ls->GetLength();
        out->n_lanes = static_cast<int>(ls->GetNumberOfLanes());
        return 0;
    }

    // Number of lanes in section `section_idx` of `road_id`; -1 on error.
    int GTRM_GetNumberOfLanesInSection(unsigned int road_id, unsigned int section_idx)
    {
        LaneSection* ls = section_of(road_id, section_idx);
        return ls == nullptr ? -1 : static_cast<int>(ls->GetNumberOfLanes());
    }

    // Fill `out` with lane `lane_idx` (vector index, not lane id) of the section. 0 / -1.
    int GTRM_GetLane(unsigned int road_id, unsigned int section_idx, unsigned int lane_idx, GTRM_Lane* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        LaneSection* ls = section_of(road_id, section_idx);
        if (ls == nullptr || lane_idx >= ls->GetNumberOfLanes())
        {
            return -1;
        }
        Lane* lane = ls->GetLaneByIdx(lane_idx);
        if (lane == nullptr)
        {
            return -1;
        }
        *out              = GTRM_Lane{};
        out->road_id      = road_id;
        out->section_idx  = section_idx;
        out->lane_id      = lane->GetId();
        out->lane_type    = static_cast<int>(lane->GetLaneType());
        out->global_id    = lane->GetGlobalId();
        out->is_road_edge = lane->IsRoadEdge() ? 1 : 0;

        LaneLink* pred = lane->GetLink(PREDECESSOR);
        if (pred != nullptr)
        {
            out->has_pred     = 1;
            out->pred_lane_id = pred->GetId();
        }
        LaneLink* succ = lane->GetLink(SUCCESSOR);
        if (succ != nullptr)
        {
            out->has_succ     = 1;
            out->succ_lane_id = succ->GetId();
        }
        return 0;
    }

    // Lateral offset (m) of the center of `lane_id` from the road reference line
    // at road distance `s`, written to `out`. 0 / -1. (roadmanager::Road::GetCenterOffset)
    int GTRM_GetLaneCenterOffset(unsigned int road_id, int lane_id, double s, double* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr)
        {
            return -1;
        }
        *out = road->GetCenterOffset(s, lane_id);
        return 0;
    }

    // Friction of `lane_id` material at road distance `s`, written to `out`.
    // 0 on success, -1 if no material/road. (roadmanager::Road::GetLaneMaterialByS)
    int GTRM_GetLaneFriction(unsigned int road_id, int lane_id, double s, double* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr)
        {
            return -1;
        }
        Lane::Material* mat = road->GetLaneMaterialByS(s, lane_id);
        if (mat == nullptr)
        {
            return -1;
        }
        *out = mat->friction;
        return 0;
    }
}
