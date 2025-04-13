# HolidayResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**comment** | Option<[**Vec<models::LocalizedText>**](LocalizedText.md)> | Additional localized comments | [optional]
**end_date** | [**chrono::NaiveDate**](chrono::NaiveDate.md) | End date of the holiday | 
**id** | [**uuid::Uuid**](uuid::Uuid.md) | Unqiue holiday id | 
**name** | [**Vec<models::LocalizedText>**](LocalizedText.md) | Localized names of the holiday | 
**nationwide** | **bool** | Is the holiday nationwide? | 
**regional_scope** | Option<[**models::RegionalScope**](RegionalScope.md)> |  | [optional]
**start_date** | [**chrono::NaiveDate**](chrono::NaiveDate.md) | Start date of the holiday | 
**subdivisions** | Option<[**Vec<models::SubdivisionReference>**](SubdivisionReference.md)> | List of subdivision references | [optional]
**temporal_scope** | Option<[**models::TemporalScope**](TemporalScope.md)> |  | [optional]
**r#type** | [**models::HolidayType**](HolidayType.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


