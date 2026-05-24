// Our own C++ shim over esmini's RoadManager C++ API to expose OpenDRIVE
// <roadMark> geometry, which the stock esminiRMLib C API does not provide.
//
// We do NOT modify anything under external/esmini (its CLAUDE.md R1: the
// EnvironmentSimulator core stays pristine). This file lives in our crate and
// links against the RoadManager static lib we already build, calling its public
// C++ classes directly.
//
// RoadManager precomputes, per road-mark type-line, the painted centerline as
// OSI points (with `endpoint` flags separating individual dashes of a broken
// line). We turn each dash into a flat triangle strip of the mark's width and
// hand it back as a flat buffer (3 doubles per vertex) plus a per-vertex color
// index (RoadMarkColor). The caller copies it out and builds a Godot mesh.

#include "gt_common.hpp"

#include <algorithm>
#include <vector>

using namespace roadmanager;

namespace
{
    // Triangle soup accumulated by the most recent GTRM_BuildRoadMarks call.
    gt::TriBuf g_marks;

    // Build a strip for one dash: centerline points pts[start..=end], offset by
    // +/- half-width perpendicular to each point's orientation (as esmini's
    // roadgeom does, via RotateY).
    void emit_dash(const std::vector<PointStruct>& pts, size_t start, size_t end, double half_w, double z_off, int color)
    {
        if (end <= start)
        {
            return;  // need at least two points
        }
        double prev_l[3] = {0, 0, 0};
        double prev_r[3] = {0, 0, 0};
        bool   have_prev = false;
        for (size_t k = start; k <= end; k++)
        {
            double l[3];
            double r[3];
            gt::offset_point(pts[k], half_w, z_off, l);
            gt::offset_point(pts[k], -half_w, z_off, r);

            if (have_prev)
            {
                g_marks.push_quad(prev_l, prev_r, r, l, color);
            }
            std::copy(l, l + 3, prev_l);
            std::copy(r, r + 3, prev_r);
            have_prev = true;
        }
    }
}  // namespace

extern "C"
{
    // Build road-mark triangles for the currently loaded network. `z_offset`
    // lifts marks above the road surface (in OpenDRIVE up/z meters). Returns the
    // number of vertices (a multiple of 3); 0 if nothing/no network loaded.
    int GTRM_BuildRoadMarks(double z_offset)
    {
        g_marks.clear();

        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return 0;
        }

        for (unsigned int ri = 0; ri < odr->GetNumOfRoads(); ri++)
        {
            Road* road = odr->GetRoadByIdx(ri);
            if (road == nullptr)
            {
                continue;
            }
            for (unsigned int si = 0; si < road->GetNumberOfLaneSections(); si++)
            {
                LaneSection* ls = road->GetLaneSectionByIdx(si);
                if (ls == nullptr)
                {
                    continue;
                }
                for (unsigned int li = 0; li < ls->GetNumberOfLanes(); li++)
                {
                    Lane* lane = ls->GetLaneByIdx(li);
                    if (lane == nullptr)
                    {
                        continue;
                    }
                    for (unsigned int mi = 0; mi < lane->GetNumberOfRoadMarks(); mi++)
                    {
                        LaneRoadMark* rm = lane->GetLaneRoadMarkByIdx(mi);
                        if (rm == nullptr || rm->GetType() == LaneRoadMark::RoadMarkType::NONE_TYPE)
                        {
                            continue;
                        }
                        int mark_color = static_cast<int>(rm->GetColor());
                        for (unsigned int ti = 0; ti < rm->GetNumberOfRoadMarkTypes(); ti++)
                        {
                            LaneRoadMarkType* rmt = rm->GetLaneRoadMarkTypeByIdx(ti);
                            if (rmt == nullptr)
                            {
                                continue;
                            }
                            for (unsigned int ni = 0; ni < rmt->GetNumberOfRoadMarkTypeLines(); ni++)
                            {
                                LaneRoadMarkTypeLine* line = rmt->GetLaneRoadMarkTypeLineByIdx(ni);
                                if (line == nullptr)
                                {
                                    continue;
                                }
                                double half_w = 0.5 * line->GetWidth();
                                if (half_w <= 0.0)
                                {
                                    half_w = 0.06;  // ~12 cm fallback when width unspecified
                                }
                                int line_color = static_cast<int>(line->GetColor());
                                // line color (if set) supersedes the mark color; 0 == UNDEFINED.
                                int color = (line_color != 0) ? line_color : mark_color;

                                std::vector<PointStruct>& pts = line->GetOSIPoints()->GetPoints();
                                // Split into dashes on `endpoint`; gaps between
                                // dashes are simply not painted.
                                size_t start = 0;
                                for (size_t q = 0; q < pts.size(); q++)
                                {
                                    bool last = (q + 1 == pts.size());
                                    if (pts[q].endpoint || last)
                                    {
                                        emit_dash(pts, start, q, half_w, z_offset, color);
                                        start = q + 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        return static_cast<int>(g_marks.vertex_count());
    }

    // Copy the built geometry: `out_xyz` needs 3*vertices doubles, `out_color`
    // needs `vertices` ints. Either may be null to skip.
    void GTRM_CopyRoadMarks(double* out_xyz, int* out_color)
    {
        g_marks.copy_out(out_xyz, out_color);
    }

    // Release the accumulated buffers.
    void GTRM_ClearRoadMarks()
    {
        g_marks.release();
    }
}

// --- Road-mark metadata (convention A: record enumeration) ------------------

// Style metadata of one <roadMark> on a lane, so the caller can pick materials.
// Enum int values follow RoadManager::LaneRoadMark:
//   type:        1 none,2 solid,3 broken,4 solid_solid,5 solid_broken,
//                6 broken_solid,7 broken_broken,8 botts_dots,9 grass,10 curb
//   weight:      0 standard,1 bold
//   color:       roadmanager RoadMarkColor (same as GTRM_BuildRoadMarks attr)
//   material:    0 standard
//   lane_change: 0 increase,1 decrease,2 both,3 none
struct GTRM_RoadMark
{
    unsigned int road_id;
    unsigned int section_idx;
    int          lane_id;
    int          type;
    int          weight;
    int          color;
    int          material;
    int          lane_change;
    double       width;
    double       height;
    double       s_offset;
    double       fade;
};

extern "C"
{
    // Number of <roadMark> entries on lane `lane_id` of (road, section); -1 on error.
    int GTRM_GetNumberOfRoadMarks(unsigned int road_id, unsigned int section_idx, int lane_id)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr || section_idx >= road->GetNumberOfLaneSections())
        {
            return -1;
        }
        LaneSection* ls = road->GetLaneSectionByIdx(section_idx);
        if (ls == nullptr)
        {
            return -1;
        }
        Lane* lane = ls->GetLaneById(lane_id);
        return lane == nullptr ? -1 : static_cast<int>(lane->GetNumberOfRoadMarks());
    }

    // Fill `out` with road-mark `mark_idx` of lane `lane_id`. 0 / -1.
    int GTRM_GetRoadMark(unsigned int road_id, unsigned int section_idx, int lane_id, unsigned int mark_idx, GTRM_RoadMark* out)
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
        if (road == nullptr || section_idx >= road->GetNumberOfLaneSections())
        {
            return -1;
        }
        LaneSection* ls = road->GetLaneSectionByIdx(section_idx);
        if (ls == nullptr)
        {
            return -1;
        }
        Lane* lane = ls->GetLaneById(lane_id);
        if (lane == nullptr || mark_idx >= lane->GetNumberOfRoadMarks())
        {
            return -1;
        }
        LaneRoadMark* rm = lane->GetLaneRoadMarkByIdx(mark_idx);
        if (rm == nullptr)
        {
            return -1;
        }
        out->road_id     = road_id;
        out->section_idx = section_idx;
        out->lane_id     = lane_id;
        out->type        = static_cast<int>(rm->GetType());
        out->weight      = static_cast<int>(rm->GetWeight());
        out->color       = static_cast<int>(rm->GetColor());
        out->material    = static_cast<int>(rm->GetMaterial());
        out->lane_change = static_cast<int>(rm->GetLaneChange());
        out->width       = rm->GetWidth();
        out->height      = rm->GetHeight();
        out->s_offset    = rm->GetSOffset();
        out->fade        = rm->GetFade();
        return 0;
    }
}
