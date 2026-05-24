// GTRM_* shim: full OpenDRIVE <signal> detail. The stock esminiRMLib C API
// (RM_GetRoadSign) returns only a handful of fields; the RoadManager Signal
// class additionally carries OSI type, country, type/subtype/value/unit/text,
// dynamic flag, and full pose. We expose all of it here.
//
// See gt_common.hpp for the shim conventions. We never modify external/esmini.

#include "gt_common.hpp"

#include <string>

using namespace roadmanager;

// One <signal>. `osi_type` is Signal::OSIType (raw int); `orientation` is
// RoadObject::Orientation (0 positive,1 negative,2 none); `dynamic` flags a
// dynamic signal (e.g. traffic light). (s,t,z_offset,h_offset,pitch,roll) are
// road-relative; (x,y,z,heading) world. String fields are library-owned and
// valid only until the next GTRM_GetSignal call (copy out promptly).
struct GTRM_Signal
{
    unsigned int road_id;
    int          id;
    unsigned int global_id;
    int          osi_type;
    int          orientation;
    int          dynamic;
    double       s;
    double       t;
    double       z_offset;
    double       h_offset;
    double       pitch;
    double       roll;
    double       x;
    double       y;
    double       z;
    double       heading;
    double       height;
    double       width;
    double       depth;
    double       length;
    double       value;
    const char*  name;
    const char*  type;
    const char*  subtype;
    const char*  country;
    const char*  value_str;
    const char*  unit;
    const char*  text;
};

namespace
{
    // Last-returned strings, kept alive for the caller to copy out promptly.
    std::string g_name;
    std::string g_type;
    std::string g_subtype;
    std::string g_country;
    std::string g_value_str;
    std::string g_unit;
    std::string g_text;
}  // namespace

extern "C"
{
    // Number of <signal> records on `road_id`; -1 on error.
    int GTRM_GetNumberOfSignals(unsigned int road_id)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        return road == nullptr ? -1 : static_cast<int>(road->GetNumberOfSignals());
    }

    // Fill `out` with signal `idx` of `road_id`. 0 / -1.
    int GTRM_GetSignal(unsigned int road_id, unsigned int idx, GTRM_Signal* out)
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
        if (road == nullptr || idx >= road->GetNumberOfSignals())
        {
            return -1;
        }
        Signal* s = road->GetSignal(idx);
        if (s == nullptr)
        {
            return -1;
        }

        // Resolve world pose from road coordinates, as RM_GetRoadSign does.
        Position pos;
        pos.SetTrackPos(road_id, s->GetS(), s->GetT());

        g_name      = s->GetName();
        g_type      = s->GetType();
        g_subtype   = s->GetSubType();
        g_country   = s->GetCountry();
        g_value_str = s->GetValueStr();
        g_unit      = s->GetUnit();
        g_text      = s->GetText();

        out->road_id     = road_id;
        out->id          = s->GetId();
        out->global_id   = s->GetGlobalId();
        out->osi_type    = s->GetOSIType();
        out->orientation = static_cast<int>(s->GetOrientation());
        out->dynamic     = s->IsDynamic() ? 1 : 0;
        out->s           = s->GetS();
        out->t           = s->GetT();
        out->z_offset    = s->GetZOffset();
        out->h_offset    = s->GetHOffset();
        out->pitch       = s->GetPitch();
        out->roll        = s->GetRoll();
        out->x           = pos.GetX();
        out->y           = pos.GetY();
        out->z           = pos.GetZ();
        out->heading     = pos.GetH();
        out->height      = s->GetHeight();
        out->width       = s->GetWidth();
        out->depth       = s->GetDepth();
        out->length      = s->GetLength();
        out->value       = s->GetValue();
        out->name        = g_name.c_str();
        out->type        = g_type.c_str();
        out->subtype     = g_subtype.c_str();
        out->country     = g_country.c_str();
        out->value_str   = g_value_str.c_str();
        out->unit        = g_unit.c_str();
        out->text        = g_text.c_str();
        return 0;
    }
}
